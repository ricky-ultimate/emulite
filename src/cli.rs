use clap::{Parser, Subcommand};

use crate::config::{DEFAULT_DISK_GB, DEFAULT_RAM_MB};

#[derive(Parser)]
#[command(
    name = "emulite",
    about = "Lightweight terminal-native Android emulator manager",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Create a new instance")]
    Create {
        name: String,
        #[arg(long, help = "Path to Android image (ISO or disk image)")]
        image: String,
        #[arg(long, default_value_t = DEFAULT_RAM_MB, help = "RAM in megabytes")]
        ram: u32,
        #[arg(long, default_value_t = DEFAULT_DISK_GB, help = "Disk size in gigabytes")]
        disk: u32,
    },
    #[command(about = "Start an instance")]
    Start { name: String },
    #[command(about = "Stop a running instance")]
    Stop { name: String },
    #[command(about = "List all instances")]
    List,
    #[command(about = "List running instances")]
    Ps,
    #[command(about = "Open an ADB shell into a running instance")]
    Shell { name: String },
    #[command(about = "Install an APK into a running instance")]
    Install { name: String, apk: String },
    #[command(about = "Destroy an instance and all its data")]
    Destroy {
        name: String,
        #[arg(long, help = "Skip confirmation prompt")]
        force: bool,
    },
}
