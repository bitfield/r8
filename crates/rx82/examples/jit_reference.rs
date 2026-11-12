// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded architectural comparison driver for `tools/check_references.py`.

use anyhow::{Context as _, Result, bail};
use r8cpu::regs::Reg;
use rx82::system::System;
use std::io::{self, BufRead as _};

fn main() -> Result<()> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let mut fields = line.split_whitespace();
        let pc: u16 = fields.next().context("missing start address")?.parse()?;
        let flags: u8 = fields.next().context("missing status")?.parse()?;
        let hex = fields.next().context("missing program")?;
        if !hex.len().is_multiple_of(2) {
            bail!("odd program length");
        }
        let mut system = System {
            turbo: true,
            ..System::default()
        };
        system.devices.clear();
        system.mem.end = 0xFFFF;
        system.mem.data.resize(0x0001_0000, 0);
        system.cpu.pc = pc;
        system.cpu.flags = flags.into();
        system.select_backend("jit")?;
        for (index, bytes) in hex.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let value = u8::from_str_radix(core::str::from_utf8(bytes)?, 16)?;
            system
                .mem
                .set(pc.wrapping_add(u16::try_from(index)?), value);
        }
        for _ in 0_u32..100_000 {
            system.tick();
            if system.cpu.halt {
                break;
            }
        }
        if !system.cpu.halt {
            bail!("program exceeded cycle limit");
        }
        println!(
            "[{},{},{},{}]",
            system.cpu.regs.get(Reg::A),
            system.cpu.pc,
            u8::from(system.cpu.flags),
            system.backend.native_entries()
        );
    }
    Ok(())
}
