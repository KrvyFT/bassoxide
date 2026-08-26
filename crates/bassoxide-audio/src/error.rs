use thiserror::Error;

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("CPAL error: {0}")]
    DeviceError(String),
    
    #[error("SoundFont error: {0}")]
    SoundFontError(String),
}
