//! Unified error type for sound-manager-core.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("registry error: {0}")]
    Registry(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(String),

    #[error("audio error: {0}")]
    Audio(String),

    #[error("sound file too long (max 30s)")]
    SoundTooLong,

    #[error("imageres patching: {0}")]
    Patching(String),

    #[error("access denied (admin required): {0}")]
    Permission(String),

    #[error("scheme error: {0}")]
    Scheme(String),

    #[error("archive error: {0}")]
    Archive(String),

    #[error("settings error: {0}")]
    Settings(String),

    #[error("translation key not found: {0}")]
    MissingTranslation(String),

    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    /// Serialization kind discriminant for typed IPC error handling.
    pub fn kind(&self) -> &'static str {
        match self {
            CoreError::Registry(_) => "Registry",
            CoreError::Io(_) => "Io",
            CoreError::Zip(_) => "Zip",
            CoreError::Audio(_) => "Audio",
            CoreError::SoundTooLong => "SoundTooLong",
            CoreError::Patching(_) => "Patching",
            CoreError::Permission(_) => "Permission",
            CoreError::Scheme(_) => "Scheme",
            CoreError::Archive(_) => "Archive",
            CoreError::Settings(_) => "Settings",
            CoreError::MissingTranslation(_) => "MissingTranslation",
            CoreError::Other(_) => "Other",
        }
    }
}
