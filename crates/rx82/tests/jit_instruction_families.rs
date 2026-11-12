// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(clippy::expect_used, reason = "test setup")]
#![expect(
    clippy::default_numeric_fallback,
    reason = "opcode literals in generated tests"
)]
#![expect(
    clippy::arbitrary_source_item_ordering,
    reason = "keep test generators beside their cases"
)]
#![expect(clippy::panic, reason = "a bounded instruction must finish")]

use r8cpu::regs::Reg;
use rx82::{state::State, system::System};

fn check(program: &[u8], flags: u8) {
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
    for system in [&mut reference, &mut native] {
        system.cpu.pc = 0x0100;
        system.cpu.flags = flags.into();
        for (reg, value) in (0_u8..8).zip([0x80, 1, 0x7F, 0xFF, 1, 1, 3, 0]) {
            system
                .cpu
                .regs
                .set(Reg::try_from(reg).expect("byte register"), value);
        }
        system.cpu.regs.set16(Reg::SP, 0x0300);
        system.mem.load(0x0100, program).expect("program fits");
        system
            .mem
            .load(0x0301, &[0xC7, 0x01, 0x02, 0x03])
            .expect("stack fits");
    }
    let mut executing = false;
    let mut native_cycles = 0_u64;
    for _ in 0_u8..64 {
        executing |= reference.cpu.state == State::Execute;
        reference.tick();
        native.tick();
        assert_eq!(
            native.cpu.state, reference.cpu.state,
            "{program:02X?} state"
        );
        assert_eq!(
            format!("{:?}", native.cpu),
            format!("{:?}", reference.cpu),
            "{program:02X?} CPU"
        );
        assert_eq!(
            format!("{:?}", native.bus),
            format!("{:?}", reference.bus),
            "{program:02X?} bus"
        );
        assert_eq!(native.mem.data, reference.mem.data, "{program:02X?} RAM");
        if executing {
            native_cycles = native_cycles.wrapping_add(1);
        }
        if executing && reference.cpu.state == State::FetchOpcode {
            assert_eq!(
                native.backend.native_entries(),
                native_cycles,
                "{program:02X?} native witness"
            );
            return;
        }
    }
    panic!("instruction did not finish: {program:02X?}");
}

macro_rules! register_test {
    ($name:ident, $opcode:expr) => {
        #[test]
        fn $name() {
            for source in 0_u8..13 {
                for target in 0_u8..13 {
                    if (source < 8) == (target < 8) {
                        for flags in [0, 0xC7] {
                            check(&[$opcode, (source << 4) | target], flags);
                        }
                    }
                }
            }
        }
    };
}

register_test!(native_register_load, 0x1E);
register_test!(native_register_add, 0x5F);
register_test!(native_register_subtract, 0x6F);
register_test!(native_register_compare, 0x7F);
register_test!(native_register_and, 0x8F);
register_test!(native_register_test, 0x9F);
register_test!(native_register_or, 0xAF);
register_test!(native_register_xor, 0xBF);

#[test]
fn native_status_and_identification() {
    for opcode in [0, 3, 4, 5, 6] {
        for flags in [0, 0xC7] {
            check(&[opcode], flags);
        }
    }
    for query in 0_u8..=255 {
        check(&[0xFA, query], 0xC7);
    }
}

#[test]
fn native_conditional_branches_and_jump() {
    for opcode in 0xF0_u8..=0xF6 {
        for displacement in [0, 1, 0x7F, 0x80, 0xFF] {
            for flags in [0, 1, 2, 0x80, 0xC7] {
                check(&[opcode, displacement], flags);
            }
        }
    }
    check(&[0xF7, 0xFF, 0xFF], 0xC7);
}

#[test]
fn native_memory_loads_and_stores() {
    for byte in 0_u8..8 {
        check(&[0x20 | byte, 0xFF, 0xBF], 0xC7);
        for word in 8_u8..13 {
            check(&[0x1D, (word << 4) | byte], 0xC7);
            check(&[0x1F, (word << 4) | byte, 0xFF], 0xC7);
            check(&[0x28, (byte << 4) | word], 0xC7);
            check(&[0x2F, (byte << 4) | word, 0xFF], 0xC7);
        }
    }
    for reg in 8_u8..13 {
        check(&[0x29, reg, 0xA5], 0xC7);
    }
}

#[test]
fn native_memory_increment_and_decrement() {
    for reg in 8_u8..13 {
        check(&[0x3D, reg], 0xC7);
        check(&[0x4D, reg], 0xC7);
    }
    for opcode in [0x3E, 0x4E] {
        check(&[opcode, 0x01, 0x03], 0xC7);
    }
}

#[test]
fn native_stack_and_returns() {
    for reg in 0_u8..12 {
        check(&[0xD0 | reg], 0xC7);
        check(&[0xE0 | reg], 0xC7);
    }
    for opcode in [0xDC, 0xEC, 7, 8] {
        check(&[opcode], 0xC7);
    }
}

#[test]
fn native_trap_and_overflow_frames() {
    for code in [0, 1, 0x3F, 0x40, 0xFF] {
        check(&[0xF9, code], 0xC7);
    }
    for flags in [0, 0xC7] {
        check(&[2], flags);
    }
}

#[test]
fn every_valid_opcode_has_a_native_execution_witness() {
    use r8cpu::instructions::{InstructionKind, Operands};
    for opcode in 0_u8..=255 {
        if let Ok(instruction) = InstructionKind::try_from(opcode) {
            let length = match instruction.operands() {
                Operands::Zero => 1,
                Operands::One => 2,
                Operands::Two => 3,
            };
            let bytes = [opcode, 0, 0];
            check(bytes.get(..length).expect("instruction fits"), 0xC7);
        }
    }
}

#[test]
fn every_invalid_opcode_uses_the_retained_rust_decoder_and_trap_path() {
    use r8cpu::instructions::InstructionKind;
    for opcode in 0_u8..=255 {
        if InstructionKind::try_from(opcode).is_ok() {
            continue;
        }
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
        for system in [&mut reference, &mut native] {
            system.cpu.pc = 0x0100;
            system.cpu.flags = 0xC7.into();
            system.cpu.regs.set16(Reg::SP, 0x0300);
            system.mem.set(0x0100, opcode);
        }
        for _ in 0_u8..11 {
            reference.tick();
            native.tick();
            assert_eq!(
                native.cpu.state, reference.cpu.state,
                "illegal opcode {opcode:02X} state"
            );
            assert_eq!(
                format!("{:?}", native.cpu),
                format!("{:?}", reference.cpu),
                "illegal CPU"
            );
            assert_eq!(
                format!("{:?}", native.bus),
                format!("{:?}", reference.bus),
                "illegal bus"
            );
            assert_eq!(native.mem.data, reference.mem.data, "illegal RAM");
        }
        assert_eq!(
            native.cpu.state,
            State::FetchOpcode,
            "illegal trap completed"
        );
        assert_eq!(native.cpu.regs.get16(Reg::SP), 0x02FC, "illegal frame size");
        assert_eq!(native.mem.get(0x02FF), 1, "return PC low byte");
        assert_eq!(native.mem.get(0x02FD), 0, "illegal trap code");
        assert_eq!(
            native.backend.native_entries(),
            0,
            "Rust decode/trap boundary"
        );
    }
}

#[test]
fn invalid_register_encodings_trap_natively() {
    for opcode in [
        0x1D, 0x1E, 0x1F, 0x28, 0x2F, 0x5F, 0x6F, 0x7F, 0x8F, 0x9F, 0xAF, 0xBF,
    ] {
        for operand in 0_u8..=255 {
            let (source, target) = (operand >> 4, operand & 15);
            // These reference helpers panic for mixed widths; literal tests cover them below.
            if matches!(opcode, 0x6F | 0x8F | 0x9F | 0xAF | 0xBF)
                && source < 13
                && target < 13
                && (source < 8) != (target < 8)
            {
                continue;
            }
            let length = if opcode == 0x1F || opcode == 0x2F {
                3
            } else {
                2
            };
            let bytes = [opcode, operand, 0xFF];
            check(bytes.get(..length).expect("instruction fits"), 0xC7);
        }
    }
    for opcode in [0x29, 0x3D, 0x4D] {
        for operand in 0_u8..=255 {
            let bytes = [opcode, operand, 0xA5];
            check(
                bytes
                    .get(..if opcode == 0x29 { 3 } else { 2 })
                    .expect("instruction fits"),
                0xC7,
            );
        }
    }
    for opcode in 0xC0_u8..=0xC9 {
        for operand in 0_u8..=255 {
            check(&[opcode, operand], 0xC7);
        }
    }
}

#[test]
fn mixed_width_reference_panics_have_literal_native_illegal_trap_frames() {
    for opcode in [0x6F, 0x8F, 0x9F, 0xAF, 0xBF] {
        for source in 0_u8..13 {
            for target in 0_u8..13 {
                if (source < 8) == (target < 8) {
                    continue;
                }
                let mut native = System {
                    turbo: true,
                    ..System::default()
                };
                native.cpu.pc = 0x0100;
                native.cpu.flags = 0xC7.into();
                native.cpu.regs.set16(Reg::SP, 0x0300);
                native
                    .mem
                    .load(0x0100, &[opcode, (source << 4) | target])
                    .expect("program fits");
                native
                    .select_backend("jit")
                    .expect("native backend available");
                for _ in 0_u8..14 {
                    native.tick();
                }
                assert_eq!(
                    native.cpu.state,
                    State::FetchOpcode,
                    "illegal trap completed"
                );
                assert_eq!(native.cpu.pc, 0, "illegal vector");
                assert_eq!(native.cpu.regs.get16(Reg::SP), 0x02FC, "trap stack size");
                for (address, value) in [(0x0300, 1), (0x02FF, 2), (0x02FE, 0xC7), (0x02FD, 0)] {
                    assert_eq!(
                        native.mem.get(address),
                        value,
                        "trap frame at {address:04X}"
                    );
                }
                assert_eq!(
                    native.backend.native_entries(),
                    9,
                    "all trap cycles are native"
                );
            }
        }
    }
}

macro_rules! shift_test {
    ($name:ident, $opcode:expr) => {
        #[test]
        fn $name() {
            for target in 0_u8..13 {
                for count in 0_u8..if $opcode & 1 == 0 { 16 } else { 8 } {
                    for flags in [0, 0xC7] {
                        check(&[$opcode, (count << 4) | target], flags);
                    }
                }
            }
        }
    };
}

shift_test!(native_shift_left_immediate, 0xC0);
shift_test!(native_shift_left_register, 0xC1);
shift_test!(native_shift_right_immediate, 0xC2);
shift_test!(native_shift_right_register, 0xC3);
shift_test!(native_arithmetic_shift_immediate, 0xC4);
shift_test!(native_arithmetic_shift_register, 0xC5);
shift_test!(native_rotate_left_immediate, 0xC6);
shift_test!(native_rotate_left_register, 0xC7);
shift_test!(native_rotate_right_immediate, 0xC8);
shift_test!(native_rotate_right_register, 0xC9);
