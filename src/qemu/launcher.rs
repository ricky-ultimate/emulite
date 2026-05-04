use std::process::{Command, Stdio};

use crate::error::{Error, Result};
use crate::instance::Instance;

fn kvm_available() -> bool {
    std::path::Path::new("/dev/kvm").exists()
        && std::fs::metadata("/dev/kvm").map(|_| true).unwrap_or(false)
}

pub fn create_disk(instance: &Instance) -> Result<()> {
    let disk_path = instance.disk_path()?;
    if disk_path.exists() {
        return Ok(());
    }

    let disk_str = disk_path
        .to_str()
        .ok_or_else(|| Error::Qemu("invalid disk path".to_string()))?;

    let status = Command::new("qemu-img")
        .args([
            "create",
            "-f",
            "qcow2",
            disk_str,
            &format!("{}G", instance.disk_gb),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(Error::Qemu(
            "qemu-img failed to create disk image".to_string(),
        ));
    }

    Ok(())
}

pub fn spawn(instance: &Instance) -> Result<u32> {
    let disk_path = instance.disk_path()?;
    let disk_str = disk_path
        .to_str()
        .ok_or_else(|| Error::Qemu("invalid disk path".to_string()))?;

    let log_path = instance.dir()?.join("qemu.log");
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| Error::Qemu(format!("failed to create qemu log file: {}", e)))?;

    let mut cmd = Command::new("qemu-system-x86_64");

    if kvm_available() {
        cmd.args(["-enable-kvm", "-cpu", "host"]);
    } else {
        cmd.args(["-cpu", "max"]);
    }

    cmd.args(["-m", &instance.ram_mb.to_string()])
        .args(["-smp", "2"])
        .args([
            "-drive",
            &format!("file={},if=virtio,format=qcow2", disk_str),
        ])
        .args(["-cdrom", &instance.image_path])
        .args(["-boot", "d"])
        .args(["-device", "virtio-net-pci,netdev=net0"])
        .args([
            "-netdev",
            &format!("user,id=net0,hostfwd=tcp::{}-:5555", instance.adb_port),
        ])
        .args([
            "-monitor",
            &format!("tcp:127.0.0.1:{},server,nowait", instance.monitor_port()),
        ])
        .args(["-display", "none"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log_file);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Qemu(format!("failed to spawn qemu: {}", e)))?;

    let pid = child.id();

    std::thread::sleep(std::time::Duration::from_millis(500));

    match child.try_wait() {
        Ok(Some(status)) => {
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            let detail = if log.trim().is_empty() {
                format!("exit status {}", status)
            } else {
                format!("exit status {}: {}", status, log.trim())
            };
            Err(Error::Qemu(format!(
                "qemu exited immediately ({}); log at {}",
                detail,
                log_path.display()
            )))
        }
        Ok(None) => Ok(pid),
        Err(e) => Err(Error::Qemu(format!("failed to poll qemu status: {}", e))),
    }
}

pub fn stop(instance: &Instance) -> Result<()> {
    use std::io::Write;
    use std::net::TcpStream;

    let addr = format!("127.0.0.1:{}", instance.monitor_port());
    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| Error::Qemu(format!("failed to connect to qemu monitor: {}", e)))?;

    stream
        .write_all(b"quit\n")
        .map_err(|e| Error::Qemu(format!("failed to send quit to monitor: {}", e)))?;

    Ok(())
}
