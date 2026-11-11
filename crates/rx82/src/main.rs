use anyhow::Result;
use clap::Parser;

use std::{fs, path::PathBuf};

use rx82::monitor::Monitor;

/// An emulator for the RX82 fantasy retro computer system.
#[derive(Debug, Parser)]
struct Args {
    /// Paths to binary files to run.
    paths: Option<Vec<PathBuf>>,
    /// Skip running boot ROM.
    #[clap(long)]
    skiprom: bool,
    /// Single-step if loading a binary file.
    #[clap(short, long)]
    step: bool,
    /// Full host-native speed.
    #[clap(short, long)]
    turbo: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut programs = Vec::new();
    if let Some(paths) = args.paths {
        for path in paths {
            let program = fs::read(path)?;
            programs.push(program);
        }
    }
    let mut mon = Monitor::default();
    if !args.skiprom {
        mon.sys.reset();
    }
    mon.step = args.step;
    mon.sys.turbo = args.turbo;
    if programs.is_empty() {
        mon.interact()?;
    } else {
        for program in programs {
            mon.run_program(&program)?;
        }
    }
    Ok(())
}
