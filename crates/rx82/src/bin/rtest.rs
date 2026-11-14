#![cfg(feature = "test")]
use anyhow::{Result, bail};
use clap::{CommandFactory as _, Parser};
use colored::Colorize as _;

use std::{fs, path::PathBuf};

use r8cpu::regs::Reg::A;
use rx82::monitor::Monitor;

/// A functional test runner for the RX82 emulator.
#[derive(Debug, Parser)]
#[command(name = "rtest")]
struct Args {
    /// Paths to functional test binaries.
    paths: Option<Vec<PathBuf>>,
}

#[expect(clippy::non_ascii_literal, reason = "it's fun")]
fn main() -> Result<()> {
    let args = Args::parse();
    let mut mon = Monitor::default();
    let mut passed = Vec::new();
    let mut failed = Vec::new();
    let Some(paths) = args.paths else {
        Args::command().print_long_help()?;
        return Ok(());
    };
    for path in paths {
        mon.sys.cpu.regs.set(A, 0x01);
        let program = fs::read(&path)?;
        mon.run_program(&program)?;
        if mon.sys.cpu.regs.get(A) != 0x00 {
            failed.push(path);
            print!("F");
        } else {
            passed.push(path);
            print!(".");
        }
    }
    println!("\n\nRan {} tests:", passed.len().strict_add(failed.len()));
    for pass in &passed {
        println!("{} {}", "✔".bright_green(), pass.display());
    }
    for fail in &failed {
        println!("{} {}", "x".bright_red(), fail.display());
    }
    println!("\n{} passed, {} failed", passed.len(), failed.len());
    if failed.is_empty() {
        Ok(())
    } else {
        bail!("some tests failed")
    }
}
