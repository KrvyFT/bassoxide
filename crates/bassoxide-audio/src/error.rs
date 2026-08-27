use thiserror::Error;

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("音频设备错误: {0}")]
    DeviceError(String),

    #[error("解码失败: {0}")]
    DecodeError(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}
