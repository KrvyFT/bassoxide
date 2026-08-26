//! GP 文件二进制读取原语。
//!
//! GP3-GP5 使用小端序 (Little-Endian) 二进制格式。
//! 本模块封装常用的读取操作，减少重复代码。

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

use crate::error::{IoError, Result};

/// 二进制数据读取器，封装 `Cursor<&[u8]>` 并提供 GP 格式专用的读取方法。
pub struct GpReader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> GpReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }

    /// 当前读取位置
    pub fn position(&self) -> usize {
        self.cursor.position() as usize
    }

    /// 剩余可读字节数
    pub fn remaining(&self) -> usize {
        let len = self.cursor.get_ref().len();
        let pos = self.cursor.position() as usize;
        len.saturating_sub(pos)
    }

    /// 跳过 N 个字节
    pub fn skip(&mut self, n: usize) -> Result<()> {
        if self.remaining() < n {
            return Err(IoError::UnexpectedEof);
        }
        self.cursor.set_position(self.cursor.position() + n as u64);
        Ok(())
    }

    // ── 基础类型读取 ──

    pub fn read_bool(&mut self) -> Result<bool> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        self.cursor.read_u8().map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        self.cursor.read_i8().map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        self.cursor
            .read_u16::<LittleEndian>()
            .map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        self.cursor
            .read_i16::<LittleEndian>()
            .map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        self.cursor
            .read_i32::<LittleEndian>()
            .map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        self.cursor
            .read_u32::<LittleEndian>()
            .map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        self.cursor
            .read_f32::<LittleEndian>()
            .map_err(|_| IoError::UnexpectedEof)
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        self.cursor
            .read_f64::<LittleEndian>()
            .map_err(|_| IoError::UnexpectedEof)
    }

    /// 读取 N 个字节到 Vec
    pub fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        if self.remaining() < n {
            return Err(IoError::UnexpectedEof);
        }
        let mut buf = vec![0u8; n];
        self.cursor.read_exact(&mut buf).map_err(|_| IoError::UnexpectedEof)?;
        Ok(buf)
    }

    // ── GP 格式专用字符串读取 ──

    /// 读取 GP 格式的 "int-byte-string"：
    /// 4 字节 int (字符串长度+1) + 实际字符串字节
    pub fn read_int_byte_string(&mut self) -> Result<String> {
        let max_len = self.read_i32()? as usize;
        if max_len == 0 {
            return Ok(String::new());
        }
        let str_len = self.read_u8()? as usize;
        let text = self.read_string_bytes(str_len)?;
        // 跳过填充字节 (max_len - 1 - str_len)
        let padding = max_len.saturating_sub(1).saturating_sub(str_len);
        if padding > 0 {
            self.skip(padding)?;
        }
        Ok(text)
    }

    /// 读取 GP 格式的 "int-string"：
    /// 4 字节 int (长度) + 字符串字节
    pub fn read_int_string(&mut self) -> Result<String> {
        let len = self.read_i32()? as usize;
        self.read_string_bytes(len)
    }

    /// 读取 GP 格式的 "byte-string"：
    /// 1 字节 (长度) + 字符串字节
    pub fn read_byte_string(&mut self) -> Result<String> {
        let len = self.read_u8()? as usize;
        self.read_string_bytes(len)
    }

    /// 读取固定长度的 byte-size-string：
    /// 1 字节 (实际长度) + N-1 字节 (填充到固定大小)
    pub fn read_byte_string_fixed(&mut self, size: usize) -> Result<String> {
        let actual_len = self.read_u8()? as usize;
        let text = self.read_string_bytes(actual_len.min(size))?;
        // 跳过填充字节
        let padding = size.saturating_sub(actual_len);
        if padding > 0 {
            self.skip(padding)?;
        }
        Ok(text)
    }

    /// 从字节读取 UTF-8 字符串（容错，无效字符替换为 ?）
    fn read_string_bytes(&mut self, len: usize) -> Result<String> {
        let bytes = self.read_bytes(len)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}
