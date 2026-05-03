use std::path::PathBuf;

use crate::error::{Error, Result};

pub const DEFAULT_RAM_MB: u32 = 2048;
pub const DEFAULT_DISK_GB: u32 = 8;
pub const ADB_BASE_PORT: u16 = 5556;

pub struct Paths;

impl Paths {
    pub fn base() -> Result<PathBuf> {
        dirs::home_dir().map(|h| h.join(".emulite")).ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found",
            ))
        })
    }

    pub fn instances() -> Result<PathBuf> {
        Ok(Self::base()?.join("instances"))
    }

    pub fn instance(name: &str) -> Result<PathBuf> {
        Ok(Self::instances()?.join(name))
    }
}
