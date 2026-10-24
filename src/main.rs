//! Lightweight OOM killer daemon for Linux
//!
//! Optimized for low-spec hardware: minimal allocations, syscalls, and binary size.

use std::ops::Not;

use linux_version::LinuxVersion;
use uname::Uname;

use crate::{error::Error, memory::lock_memory_pages, monitor::Monitor};

mod cli;
mod daemon;
mod errno;
mod error;
mod kill;
mod linux_version;
mod memory;
mod monitor;
mod process;
mod uname;
mod utils;

/// Minimum Linux version with PSI support
const LINUX_4_20: LinuxVersion = LinuxVersion { major: 4, minor: 20 };

fn main() -> error::Result<()> {
    let args = cli::parse_args()?;
    let should_daemonize = args.no_daemon.not();

    // Verify kernel version once at startup
    let uname = Uname::new()?;
    uname.print_info()?;
    let version = uname.parse_version().unwrap_or(LinuxVersion { major: 0, minor: 0 });
    if version < LINUX_4_20 {
        eprintln!("{version} does not meet minimum requirements (Linux 4.20+)");
        return Err(Error::InvalidLinuxVersion);
    }

    // Pre-allocated buffers (stack, no heap allocation)
    let proc_buf = [0_u8; 50];
    let buf = [0_u8; 100];

    if should_daemonize {
        println!("\nStarting daemonization process!");
        daemon::daemonize()?;
    }

    // Lock memory pages to avoid swap thrashing
    if let Err(err) = lock_memory_pages() {
        eprintln!("Failed to lock memory pages: {:?}. Continuing anyway.", err);
    } else {
        println!("Memory pages locked!");
        eprintln!("Memory pages locked!");
    }

    Monitor::new(proc_buf, buf, args)?.poll()
}