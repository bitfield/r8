// SPDX-FileCopyrightText: 2026 John Arundel
// SPDX-FileCopyrightText: 2026 Zayn Otley (modifications)
// https://ZaynOtley.com
// https://IntuitionEngine.io

use anyhow::Result;
use clap::Parser;

use std::{fs, path::PathBuf};

use rx82::monitor::Monitor;

/// An emulator for the RX82 fantasy retro computer system.
#[derive(Debug, Parser)]
#[expect(clippy::struct_excessive_bools, reason = "independent CLI switches")]
struct Args {
    /// CPU execution backend: interpreter or jit.
    #[clap(long, default_value = if cfg!(r8jit_native) { "jit" } else { "interpreter" })]
    backend: String,
    /// Pace execution to the nominal 4 MHz clock.
    #[clap(long)]
    no_turbo: bool,
    /// Paths to binary files to run.
    paths: Option<Vec<PathBuf>>,
    /// Skip running boot ROM.
    #[clap(long)]
    skiprom: bool,
    /// Single-step if loading a binary file.
    #[clap(short, long)]
    step: bool,
    /// Full host-native speed (default).
    #[clap(short, long, default_value_t = true, conflicts_with = "no_turbo")]
    turbo: bool,
}

#[cfg_attr(
    not(test),
    expect(clippy::single_call_fn, reason = "test startup clock selection")
)]
fn configure_monitor(mon: &mut Monitor, args: &Args) -> Result<()> {
    mon.sys.select_backend(&args.backend)?;
    mon.sys.turbo = args.turbo && !args.no_turbo;
    if !args.skiprom {
        mon.sys.reset();
    }
    mon.step = args.step;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut programs = Vec::new();
    if let Some(paths) = args.paths.as_ref() {
        for path in paths {
            let program = fs::read(path)?;
            programs.push(program);
        }
    }
    let mut mon = Monitor::default();
    configure_monitor(&mut mon, &args)?;
    if programs.is_empty() {
        mon.interact()?;
    } else {
        for program in programs {
            mon.run_program(&program)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "test setup")]
#[expect(clippy::std_instead_of_alloc, reason = "test harness uses std")]
mod tests {
    use super::{Args, configure_monitor};
    use clap::Parser as _;
    use core::cell::Cell;
    use rx82::{bus::Bus, monitor::Monitor, system::Device};
    use std::rc::Rc;

    struct ClockProbe(Rc<Cell<u64>>);

    impl Device for ClockProbe {
        fn tick(&mut self, _: &mut Bus) {
            self.0.set(self.0.get().wrapping_add(1));
        }
    }

    #[test]
    fn command_line_speed_applies_to_boot() {
        for (arguments, turbo) in [
            (vec!["rx82"], true),
            (vec!["rx82", "--turbo"], true),
            (vec!["rx82", "-t"], true),
            (vec!["rx82", "--no-turbo"], false),
        ] {
            let args = Args::try_parse_from(&arguments).expect("valid speed option");
            let calls = Rc::new(Cell::new(0));
            let mut monitor = Monitor::default();
            monitor.sys.clock = Box::new(ClockProbe(Rc::clone(&calls)));
            configure_monitor(&mut monitor, &args).expect("configure monitor");
            assert_eq!(monitor.sys.turbo, turbo, "{arguments:?}");
            assert_eq!(calls.get() == 0, turbo, "boot clock calls: {arguments:?}");
        }
    }

    #[test]
    fn command_line_rejects_conflicting_speed_flags() {
        assert!(matches!(
            Args::try_parse_from(["rx82", "--turbo", "--no-turbo"]),
            Err(error) if error.kind() == clap::error::ErrorKind::ArgumentConflict
        ));
    }
}
