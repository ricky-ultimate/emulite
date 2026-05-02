use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("instance '{0}' not found")]
    InstanceNotFound(String),
    #[error("instance '{0}' already exists")]
    InstanceAlreadyExists(String),
    #[error("instance '{0}' is already running")]
    InstanceAlreadyRunning(String),
    #[error("instance '{0}' is not running")]
    InstanceNotRunning(String),
    #[error("image not found: {0}")]
    ImageNotFound(String),
    #[error("no available ports in range")]
    NoAvailablePorts,
    #[error("qemu: {0}")]
    Qemu(String),
    #[error("adb: {0}")]
    Adb(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("{0}")]
    TomlDe(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
