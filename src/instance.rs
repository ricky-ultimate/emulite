use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::{ADB_BASE_PORT, Paths};
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Stopped,
    Running,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Stopped => write!(f, "stopped"),
            State::Running => write!(f, "running"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub image_path: String,
    pub ram_mb: u32,
    pub disk_gb: u32,
    pub adb_port: u16,
    pub state: State,
    pub pid: Option<u32>,
}

impl Instance {
    pub fn new(name: String, image_path: String, ram_mb: u32, disk_gb: u32, adb_port: u16) -> Self {
        Self {
            name,
            image_path,
            ram_mb,
            disk_gb,
            adb_port,
            state: State::Stopped,
            pid: None,
        }
    }

    pub fn dir(&self) -> Result<PathBuf> {
        Paths::instance(&self.name)
    }

    pub fn config_path(&self) -> Result<PathBuf> {
        Ok(self.dir()?.join("instance.toml"))
    }

    pub fn disk_path(&self) -> Result<PathBuf> {
        Ok(self.dir()?.join("disk.qcow2"))
    }

    pub fn monitor_port(&self) -> u16 {
        self.adb_port + 1
    }

    pub fn is_alive(&self) -> bool {
        let Some(pid) = self.pid else {
            return false;
        };
        let status_path = format!("/proc/{}/status", pid);
        match std::fs::read_to_string(&status_path) {
            Ok(content) => !content
                .lines()
                .any(|line| line.starts_with("State:") && line.contains('Z')),
            Err(_) => false,
        }
    }

    pub fn load(name: &str) -> Result<Self> {
        let path = Paths::instance(name)?.join("instance.toml");
        if !path.exists() {
            return Err(Error::InstanceNotFound(name.to_string()));
        }
        let raw = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }

    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(self.dir()?)?;
        std::fs::write(self.config_path()?, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let dir = self.dir()?;
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    pub fn list() -> Result<Vec<Self>> {
        let dir = Paths::instances()?;
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut instances: Vec<Self> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter_map(|name| Self::load(&name).ok())
            .collect();
        instances.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(instances)
    }
}

pub fn next_available_port() -> Result<u16> {
    let used: HashSet<u16> = Instance::list()?
        .into_iter()
        .flat_map(|i| [i.adb_port, i.monitor_port()])
        .collect();

    let mut port = ADB_BASE_PORT;
    loop {
        if !used.contains(&port) && !used.contains(&(port + 1)) {
            return Ok(port);
        }
        port = port.checked_add(2).ok_or(Error::NoAvailablePorts)?;
    }
}
