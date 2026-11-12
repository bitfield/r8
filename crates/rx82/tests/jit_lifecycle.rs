// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(clippy::expect_used, reason = "test setup")]

use r8cpu::regs::Reg;
use rx82::{bus::Bus, cpu::Cpu, system::System};

#[test]
fn changing_operands_reclaims_cache_without_losing_native_coverage() {
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system
        .select_backend("jit")
        .expect("native backend available");
    for operand in 0_u8..=255 {
        system.cpu = Cpu::default();
        system.bus = Bus::default();
        system.cpu.pc = 0x0100;
        system
            .mem
            .load(0x0100, &[0x10, operand])
            .expect("program fits");
        for _ in 0_u8..6 {
            system.tick();
        }
        let (blocks, bytes) = system.backend.cache_usage();
        assert_eq!(
            blocks,
            usize::from(operand).saturating_add(1).min(64),
            "evict only one entry when full"
        );
        assert!(bytes <= 64 * 4096, "byte bound");
        assert_eq!(system.cpu.regs.get(Reg::A), operand, "new operand");
        assert_eq!(
            system.backend.native_entries(),
            u64::from(operand).wrapping_add(1),
            "no budget fallback"
        );
    }
}

#[test]
fn explicit_clear_and_reset_release_old_entries() {
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system.devices.clear();
    system.mem.end = 0xFFFF;
    system.mem.data.resize(0x0001_0000, 0);
    system
        .select_backend("jit")
        .expect("native backend available");
    system.cpu.pc = 0x0100;
    system.mem.load(0x0100, &[1, 0]).expect("program fits");
    for _ in 0_u8..8 {
        system.tick();
    }
    assert_eq!(
        system.backend.cache_usage(),
        (2, 8192),
        "NOP and HALT cached"
    );
    system.backend.clear_cache().expect("clear succeeds");
    assert_eq!(
        system.backend.cache_usage(),
        (0, 0),
        "all mappings released"
    );
    system.cpu = Cpu::default();
    system.bus = Bus::default();
    system.cpu.pc = 0x0100;
    for _ in 0_u8..8 {
        system.tick();
    }
    system.mem.load(0xFFFE, &[0, 2]).expect("reset vector fits");
    system.reset();
    assert_eq!(
        system.backend.cache_usage(),
        (1, 4096),
        "reset retains only its new HALT block"
    );
    assert_eq!(system.cpu.pc, 0x0201, "reset vector was used");
}
