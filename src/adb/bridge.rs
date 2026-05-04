use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

fn serial(port: u16) -> String {
    format!("127.0.0.1:{}", port)
}

pub fn connect(port: u16) -> Result<()> {
    let output = Command::new("adb")
        .args(["connect", &serial(port)])
        .output()
        .map_err(|e| Error::Adb(format!("failed to run adb: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains("failed") || stdout.contains("cannot") || stdout.contains("error") {
        return Err(Error::Adb(format!("adb connect failed: {}", stdout.trim())));
    }

    Ok(())
}

pub fn wait_for_device(port: u16) -> Result<()> {
    let status = Command::new("adb")
        .args(["-s", &serial(port), "wait-for-device"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| Error::Adb(format!("failed to run adb wait-for-device: {}", e)))?;

    if !status.success() {
        return Err(Error::Adb("adb wait-for-device failed".to_string()));
    }

    Ok(())
}

pub fn disconnect(port: u16) -> Result<()> {
    let _ = Command::new("adb")
        .args(["disconnect", &serial(port)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

pub fn install(port: u16, apk: &Path) -> Result<()> {
    let apk_str = apk
        .to_str()
        .ok_or_else(|| Error::Adb("invalid apk path".to_string()))?;

    let status = Command::new("adb")
        .args(["-s", &serial(port), "install", "-r", apk_str])
        .status()
        .map_err(|e| Error::Adb(format!("failed to run adb install: {}", e)))?;

    if !status.success() {
        return Err(Error::Adb("adb install failed".to_string()));
    }

    Ok(())
}

pub fn shell(port: u16) -> Result<()> {
    Command::new("adb")
        .args(["-s", &serial(port), "shell"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| Error::Adb(format!("failed to spawn adb shell: {}", e)))?;
    Ok(())
}
