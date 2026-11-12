// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(clippy::expect_used, reason = "native test setup")]

use r8cpu::regs::Reg;
use rx82::{bus::Bus, cpu::Cpu, system::System};

macro_rules! family_test {
    ($name:ident, $opcode:expr, $unary:expr, $word:expr) => {
        #[test]
        fn $name() {
            check_family($opcode, $unary, $word);
        }
    };
}

fn check_family(base: u8, unary: bool, word: bool) {
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    native
        .select_backend("jit")
        .expect("native backend available");
    for register in if word { 8_u8..13 } else { 0_u8..8 } {
        for value in if word {
            [0_u16, 0x7FFF, 0x8000, 0xFFFF]
        } else {
            [0, 0x7F, 0x80, 0xFF]
        } {
            for operand in if word {
                [0_u16, 1, 0x8000, 0xFFFF]
            } else {
                [0, 1, 0x80, 0xFF]
            } {
                for flags in [0x04, 0xC7] {
                    let before_entries = native.backend.native_entries();
                    for system in [&mut reference, &mut native] {
                        system.cpu = Cpu::default();
                        system.bus = Bus::default();
                        system.cpu.pc = 0x0100;
                        system.cpu.flags = flags.into();
                        let reg = Reg::try_from(register).expect("valid register");
                        if word {
                            system.cpu.regs.set16(reg, value);
                        } else {
                            system.cpu.regs.set(reg, value.to_le_bytes()[0]);
                        }
                        let [lo, hi] = operand.to_le_bytes();
                        system.mem.set(0x0100, base | register);
                        system.mem.set(0x0101, lo);
                        system.mem.set(0x0102, hi);
                    }
                    for _ in 0_u8..if unary {
                        4
                    } else if word {
                        8
                    } else {
                        6
                    } {
                        reference.tick();
                        native.tick();
                        assert_eq!(
                            native.cpu.state, reference.cpu.state,
                            "opcode {base:02X} state"
                        );
                        assert_eq!(
                            format!("{:?}", native.cpu),
                            format!("{:?}", reference.cpu),
                            "opcode {base:02X} CPU"
                        );
                        assert_eq!(
                            format!("{:?}", native.bus),
                            format!("{:?}", reference.bus),
                            "opcode {base:02X} bus"
                        );
                    }
                    assert_eq!(native.mem.data, reference.mem.data, "opcode {base:02X} RAM");
                    assert_eq!(
                        native.backend.native_entries(),
                        before_entries.wrapping_add(1),
                        "opcode {base:02X} register {register}"
                    );
                }
            }
        }
    }
}

family_test!(native_byte_increment, 0x30, true, false);
family_test!(native_byte_load, 0x10, false, false);
family_test!(native_byte_add, 0x50, false, false);
family_test!(native_byte_decrement, 0x40, true, false);
family_test!(native_byte_subtract, 0x60, false, false);
family_test!(native_byte_compare, 0x70, false, false);
family_test!(native_byte_and, 0x80, false, false);
family_test!(native_byte_test, 0x90, false, false);
family_test!(native_byte_or, 0xA0, false, false);
family_test!(native_byte_xor, 0xB0, false, false);

family_test!(native_word_load, 0x10, false, true);
family_test!(native_word_add, 0x50, false, true);
family_test!(native_word_increment, 0x30, true, true);
family_test!(native_word_decrement, 0x40, true, true);
family_test!(native_word_subtract, 0x60, false, true);
family_test!(native_word_compare, 0x70, false, true);
family_test!(native_word_and, 0x80, false, true);
family_test!(native_word_test, 0x90, false, true);
family_test!(native_word_or, 0xA0, false, true);
family_test!(native_word_xor, 0xB0, false, true);
