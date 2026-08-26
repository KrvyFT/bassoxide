//! bassoxide-io 错误类型定义。

use thiserror::Error;

/// I/O 操作错误
#[derive(Debug, Error)]
pub enum IoError {
    #[error("文件读取失败: {0}")]
    ReadFailed(#[from] std::io::Error),

    #[error("不支持的文件格式: {0}")]
    UnsupportedFormat(String),

    #[error("版本不兼容: {0}")]
    IncompatibleVersion(String),

    #[error("数据损坏: {msg} (偏移量 0x{offset:X})")]
    CorruptData { msg: String, offset: usize },

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("意外的数据结尾")]
    UnexpectedEof,
}

pub type Result<T> = std::result::Result<T, IoError>;
