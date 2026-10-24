//! Command-line argument parsing - zero dependency, minimal allocations

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy)]
pub struct CommandLineArgs {
    pub verbose: bool,
    pub no_daemon: bool,
    pub cutoff_psi: f32,
}

impl Default for CommandLineArgs {
    fn default() -> Self {
        Self {
            verbose: false,
            no_daemon: false,
            cutoff_psi: 25.0,
        }
    }
}

pub fn parse_args() -> Result<CommandLineArgs> {
    let mut args = CommandLineArgs::default();
    let mut args_iter = std::env::args().skip(1).peekable();

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "-V" | "--verbose" => args.verbose = true,
            "-n" | "--no-daemon" => args.no_daemon = true,
            "-p" | "--psi" => {
                let val = args_iter.next().ok_or(Error::ParseFloat)?;
                args.cutoff_psi = val.parse()?;
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {arg}");
                print_help();
                return Err(Error::ParseFloat);
            }
        }
    }

    Ok(args)
}

fn print_help() {
    eprintln!(
        "bustd - Lightweight OOM killer daemon

Usage: bustd [OPTIONS]

Options:
  -V, --verbose      Enable verbose output
  -n, --no-daemon    Run in foreground (for systemd)
  -p, --psi <FLOAT>  PSI cutoff percentage (default: 25.0)
  -h, --help         Show this help
"
    );
}