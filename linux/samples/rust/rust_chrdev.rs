// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.

use core::result::Result::Err;

use kernel::io_buffer::{IoBufferReader, IoBufferWriter};
use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::{chrdev, file};

const GLOBALMEM_SIZE: usize = 0x1000;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Rust for Linux Contributors",
    description: "Rust character device sample",
    license: "GPL",
}

static GLOBALMEM_BUF: Mutex<[u8; GLOBALMEM_SIZE]> = unsafe { Mutex::new([0u8; GLOBALMEM_SIZE]) };

struct RustFile {
    #[allow(dead_code)]
    inner: &'static Mutex<[u8; GLOBALMEM_SIZE]>,
}

#[vtable]
impl file::Operations for RustFile {
    type Data = Box<Self>;

    fn open(_shared: &(), _file: &file::File) -> Result<Box<Self>> {
        Ok(Box::try_new(RustFile {
            inner: &GLOBALMEM_BUF,
        })?)
    }

    fn write(
        this: &Self,
        _file: &file::File,
        reader: &mut impl IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        // 计算可以从 IoBufferReader 读取并写入全局缓冲区的最大字节数
        let buff_size = reader.len().min(GLOBALMEM_SIZE as usize);

        // 加锁以保证线程安全地访问全局内存缓冲区
        {
            let mut inner = this.inner.lock();
            // 从 IoBufferReader 读取数据到全局内存缓冲区
            reader.read_slice(&mut inner[..buff_size])?;
        }

        // 返回成功写入的字节数
        Ok(buff_size as usize)
    }

    // 从全局内存缓冲区读取数据到 IoBufferWriter
    fn read(
        this: &Self,
        _file: &file::File,
        writer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        // 如果 IoBufferWriter 已经为空，则无需读取，直接返回 0
        if writer.is_empty() {
            return Ok(0);
        }

        // 转换偏移量为 usize 类型，并计算可读的最大字节数
        let offset = offset as usize;
        let buff_size = writer.len().min((GLOBALMEM_SIZE - offset).into());

        // 加锁以保证线程安全地访问全局内存缓冲区
        {
            let inner = this.inner.lock();
            // 从全局内存缓冲区读取数据并写入 IoBufferWriter
            writer.write_slice(&inner[offset..offset + buff_size])?;
        }

        // 返回成功读取的字节数
        Ok(buff_size)
    }
}

struct RustChrdev {
    _dev: Pin<Box<chrdev::Registration<2>>>,
}

impl kernel::Module for RustChrdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;

        // Register the same kind of device twice, we're just demonstrating
        // that you can use multiple minors. There are two minors in this case
        // because its type is `chrdev::Registration<2>`
        chrdev_reg.as_mut().register::<RustFile>()?;
        chrdev_reg.as_mut().register::<RustFile>()?;

        Ok(RustChrdev { _dev: chrdev_reg })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
