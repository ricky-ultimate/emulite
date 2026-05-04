mod adb;
mod cli;
mod config;
mod error;
mod instance;
mod qemu;

use std::path::Path;

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::error::{Error, Result};
use crate::instance::{Instance, State};

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Create {
            name,
            image,
            ram,
            disk,
        } => cmd_create(name, image, ram, disk),
        Commands::Start { name } => cmd_start(name),
        Commands::Stop { name } => cmd_stop(name),
        Commands::List => cmd_list(),
        Commands::Ps => cmd_ps(),
        Commands::Shell { name } => cmd_shell(name),
        Commands::Install { name, apk } => cmd_install(name, apk),
        Commands::Destroy { name, force } => cmd_destroy(name, force),
    }
}

fn cmd_create(name: String, image: String, ram: u32, disk: u32) -> Result<()> {
    if Instance::load(&name).is_ok() {
        return Err(Error::InstanceAlreadyExists(name));
    }

    let image_path = Path::new(&image);
    if !image_path.exists() {
        return Err(Error::ImageNotFound(image));
    }

    let canonical = image_path.canonicalize()?.to_string_lossy().into_owned();

    let port = instance::next_available_port()?;
    let inst = Instance::new(name.clone(), canonical, ram, disk, port);

    std::fs::create_dir_all(inst.dir()?)?;
    qemu::launcher::create_disk(&inst)?;
    inst.save()?;

    println!("created instance '{}' on adb port {}", name, port);
    Ok(())
}

fn cmd_start(name: String) -> Result<()> {
    let mut inst = Instance::load(&name)?;

    if inst.state == State::Running && inst.is_alive() {
        return Err(Error::InstanceAlreadyRunning(name));
    }

    let pid = qemu::launcher::spawn(&inst)?;
    inst.state = State::Running;
    inst.pid = Some(pid);
    inst.save()?;

    println!(
        "started '{}' (pid {}, adb port {})",
        name, pid, inst.adb_port
    );
    println!(
        "run 'emulite shell {}' once android has finished booting",
        name
    );
    Ok(())
}

fn cmd_stop(name: String) -> Result<()> {
    let mut inst = Instance::load(&name)?;

    if inst.state != State::Running || !inst.is_alive() {
        return Err(Error::InstanceNotRunning(name));
    }

    qemu::launcher::stop(&inst)?;
    adb::bridge::disconnect(inst.adb_port)?;

    inst.state = State::Stopped;
    inst.pid = None;
    inst.save()?;

    println!("stopped '{}'", name);
    Ok(())
}

fn cmd_list() -> Result<()> {
    let instances = Instance::list()?;

    if instances.is_empty() {
        println!("no instances found");
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<10} {:<6} {}",
        "NAME", "STATE", "RAM (MB)", "PORT", "IMAGE"
    );
    println!("{}", "-".repeat(72));

    for inst in instances {
        let effective_state = if inst.state == State::Running && !inst.is_alive() {
            "dead".to_string()
        } else {
            inst.state.to_string()
        };
        let image_name = Path::new(&inst.image_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| inst.image_path.clone());
        println!(
            "{:<20} {:<10} {:<10} {:<6} {}",
            inst.name, effective_state, inst.ram_mb, inst.adb_port, image_name
        );
    }

    Ok(())
}

fn cmd_ps() -> Result<()> {
    let running: Vec<Instance> = Instance::list()?
        .into_iter()
        .filter(|i| i.state == State::Running && i.is_alive())
        .collect();

    if running.is_empty() {
        println!("no running instances");
        return Ok(());
    }

    println!("{:<20} {:<8} {}", "NAME", "PID", "PORT");
    println!("{}", "-".repeat(36));

    for inst in running {
        let pid = inst
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("{:<20} {:<8} {}", inst.name, pid, inst.adb_port);
    }

    Ok(())
}

fn cmd_shell(name: String) -> Result<()> {
    let inst = Instance::load(&name)?;

    if inst.state != State::Running || !inst.is_alive() {
        return Err(Error::InstanceNotRunning(name));
    }

    adb::bridge::connect(inst.adb_port)?;
    println!("waiting for device to come online...");
    adb::bridge::wait_for_device(inst.adb_port)?;
    adb::bridge::shell(inst.adb_port)
}

fn cmd_install(name: String, apk: String) -> Result<()> {
    let inst = Instance::load(&name)?;

    if inst.state != State::Running || !inst.is_alive() {
        return Err(Error::InstanceNotRunning(name));
    }

    let apk_path = Path::new(&apk);
    if !apk_path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("apk not found: {}", apk),
        )));
    }

    adb::bridge::connect(inst.adb_port)?;
    adb::bridge::install(inst.adb_port, apk_path)?;
    println!("installed '{}' on '{}'", apk, name);
    Ok(())
}

fn cmd_destroy(name: String, force: bool) -> Result<()> {
    let inst = Instance::load(&name)?;

    if !force {
        use std::io::Write;
        print!("destroy instance '{}' and delete all data? [y/N] ", name);
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("aborted");
            return Ok(());
        }
    }

    if inst.state == State::Running && inst.is_alive() {
        qemu::launcher::stop(&inst)?;
        adb::bridge::disconnect(inst.adb_port)?;
    }

    inst.delete()?;
    println!("destroyed '{}'", name);
    Ok(())
}
