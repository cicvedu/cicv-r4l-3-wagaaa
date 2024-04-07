// SPDX-License-Identifier: GPL-2.0

//! Rust completion device sample.

use kernel::prelude::*;
use kernel::{
    chrdev,
    file::{self, File},
    io_buffer::{IoBufferReader, IoBufferWriter},
    mutex_init,
    str::CStr,
    sync::{CondVar, Mutex},
    task::Task,
};

module! {
    type: RustCompletion,
    name: "completion",
    author: "wagaaa",
    description: "Example of Kernel's completion mechanism in Rust",
    license: "GPL",
}

kernel::init_static_sync! {
    static SAMPLE_MUTEX: Mutex<bool> = false;
    static SAMPLE_CONDVAR: CondVar;
}

struct CplDev;
#[vtable]
impl file::Operations for CplDev {
    type Data = ();

    fn open(_shared: &(), _file: &File) -> Result<Self::Data> {
        pr_info!("completion_open() is invoked\n");
        Ok(())
    }

    fn read(_shared: (), _: &File, data: &mut impl IoBufferWriter, _offset: u64) -> Result<usize> {
        pr_info!("completion_read() is invoked\n");
        if data.is_empty() {
            return Ok(0);
        }

        let mut lock = SAMPLE_MUTEX.lock();
        while !*lock {
            SAMPLE_CONDVAR.wait(&mut lock)?;
        }

        *lock = false;
        SAMPLE_CONDVAR.notify_one();
        SAMPLE_CONDVAR.notify_all();
        SAMPLE_CONDVAR.free_waiters();

        pr_info!(
            "awoken {}({})\n",
            Task::current().pid(),
            CStr::from_char_ptr(Task::current().comm().as_ptr())
        );

        Ok(0)
    }

    fn write(
        _shared: (),
        _: &File,
        _data: &mut impl IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("completion_write() is invoked\n");

        let mut lock = SAMPLE_MUTEX.lock();
        *lock = true;

        SAMPLE_CONDVAR.notify_one();
        SAMPLE_CONDVAR.notify_all();
        SAMPLE_CONDVAR.free_waiters();

        pr_info!(
            "process {}({}) awakening the readers...\n",
            Task::current().pid(),
            CStr::from_char_ptr(Task::current().comm().as_ptr())
        );

        Ok(0) // Return 0 because we're not actually writing any data
    }
}

struct RustCdev {
    _dev: Pin<Box<chrdev::Registration<1>>>,
}

impl kernel::Module for RustCdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("{name} is loaded (init)\n");

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;
        chrdev_reg.as_mut().register::<CplDev>()?;

        Ok(RustCdev { _dev: chrdev_reg })
    }
}

impl Drop for RustCdev {
    fn drop(&mut self) {
        pr_info!("completion unloaded (exit)\n");
    }
}
