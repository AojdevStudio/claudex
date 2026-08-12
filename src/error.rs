use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClaudexError {
    #[error("invalid arguments: {0}")]
    Arguments(String),
    #[error("cannot read configuration at {path}: {source}")]
    ConfigRead { path: PathBuf, source: io::Error },
    #[error("invalid configuration at {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("configuration error: {0}")]
    Config(String),
    #[error("secret error: {0}")]
    Secret(String),
    #[error("Claude Code is not available: {0}")]
    ClaudeNotFound(String),
    #[error("cannot launch Claude Code: {0}")]
    Launch(io::Error),
    #[error("proxy check failed: {0}")]
    DoctorNetwork(String),
    #[error("live inference check failed: {0}")]
    DoctorLive(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl ClaudexError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Arguments(_)
            | Self::ConfigRead { .. }
            | Self::ConfigParse { .. }
            | Self::Config(_)
            | Self::Secret(_)
            | Self::ClaudeNotFound(_) => 2,
            Self::DoctorNetwork(_) => 3,
            Self::DoctorLive(_) => 4,
            Self::Launch(_) | Self::Io(_) => 1,
        }
    }
}
