// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(clippy::expect_used, reason = "native test setup")]

use rx82::system::System;

#[test]
fn bounded_regions_match_every_cycle_budget() {
    // LD A,3; DEC A; BNE -3; HALT.
    let program = [0x10, 3, 0x40, 0xF2, 0xFD, 0];
    for budget in 0_u32..=48 {
        let mut reference = System {
            turbo: true,
            ..System::default()
        };
        let mut native = System {
            turbo: true,
            ..System::default()
        };
        native.select_backend("jit").expect("JIT");
        for system in [&mut reference, &mut native] {
            system.cpu.pc = 0x100;
            system.mem.load(0x100, &program).expect("program fits");
        }
        let mut expected = 0_u32;
        while expected < budget && !reference.cpu.halt {
            reference.tick();
            expected = expected.wrapping_add(1);
        }
        assert_eq!(native.run_cycles(budget), expected, "budget {budget}");
        assert_eq!(
            format!("{:?}", native.cpu),
            format!("{:?}", reference.cpu),
            "CPU budget {budget}"
        );
        assert_eq!(
            format!("{:?}", native.bus),
            format!("{:?}", reference.bus),
            "bus budget {budget}"
        );
        assert_eq!(native.cycles, reference.cycles, "cycles budget {budget}");
        assert_eq!(native.mem.data, reference.mem.data, "RAM budget {budget}");
        if budget == 48 && !cfg!(feature = "states") {
            assert_eq!(
                native.backend.native_entries(),
                u64::from(expected),
                "fetch and decode also execute natively"
            );
        }
    }
}

#[test]
fn changed_code_and_rom_execute_without_stale_regions() {
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system.select_backend("jit").expect("JIT");
    for value in [3_u8, 200, 0, 255] {
        system.cpu = rx82::cpu::Cpu::default();
        system.cpu.pc = 0x100;
        system.bus = rx82::bus::Bus::default();
        system.mem.load(0x100, &[0x10, value, 0]).expect("code");
        assert_eq!(system.run_cycles(100), 10);
        assert_eq!(system.cpu.regs.get(r8cpu::regs::Reg::A), value);
    }
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    native.select_backend("jit").expect("JIT");
    for machine in [&mut reference, &mut native] {
        machine.devices.insert(
            0,
            Box::new(rx82::rom::Rom {
                start: 0xC100,
                end: 0xC102,
                data: vec![0x10, 99, 0],
            }),
        );
        machine.cpu.pc = 0xC100;
    }
    assert_eq!(reference.run_cycles(100), 10);
    assert_eq!(native.run_cycles(100), 10);
    assert_eq!(format!("{:?}", native.cpu), format!("{:?}", reference.cpu));
    assert_eq!(format!("{:?}", native.bus), format!("{:?}", reference.bus));
}

#[test]
fn custom_devices_and_debugging_keep_every_tick_observable() {
    extern crate alloc;
    use alloc::rc::Rc;
    use core::cell::Cell;
    struct Observer(Rc<Cell<u32>>);
    impl rx82::system::Device for Observer {
        fn tick(&mut self, _: &mut rx82::bus::Bus) {
            self.0.set(self.0.get().wrapping_add(1));
        }
    }
    let observed = Rc::new(Cell::new(0));
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system
        .devices
        .push(Box::new(Observer(Rc::clone(&observed))));
    system.select_backend("jit").expect("JIT");
    system.cpu.pc = 0x100;
    system.mem.load(0x100, &[1, 1, 0]).expect("code");
    assert_eq!(system.run_cycles(100), 12);
    assert_eq!(observed.get(), 12);
    system.cpu = rx82::cpu::Cpu::default();
    system.cpu.pc = 0x100;
    system.bus = rx82::bus::Bus::default();
    system.devices.pop();
    system.debug = true;
    assert_eq!(system.run_cycles(100), 12);
    assert_eq!(system.history.len(), 12);
}

#[test]
#[ignore = "release performance measurement after compatibility checks"]
fn bounded_workload_performance() {
    use std::time::Instant;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["countdown", "memcpy"] {
        let code = r8asm::assemble_source_file(root.join(format!("../r8asm/examples/{name}.asm")))
            .expect("workload");
        for sample in 0_u8..5 {
            let mut reference = System {
                turbo: true,
                ..System::default()
            };
            let mut native = System {
                turbo: true,
                ..System::default()
            };
            native.select_backend("jit").expect("JIT");
            for system in [&mut reference, &mut native] {
                system.cpu.pc = 0x100;
                system.mem.load(0x100, &code).expect("code");
                for value in 0_u8..=254 {
                    system
                        .mem
                        .set(0x1000_u16.wrapping_add(u16::from(value)), value);
                }
            }
            let initial_memory = native.mem.data.clone();
            let run = |machine: &mut System| {
                let mut cycles = 0_u32;
                while !machine.cpu.halt && cycles < 15_000_000 {
                    cycles = cycles.wrapping_add(machine.run_cycles(4096));
                }
                cycles
            };
            let compare = |machine: &System, baseline: &System| {
                assert!(baseline.cpu.halt && machine.cpu.halt);
                assert_eq!(format!("{:?}", machine.cpu), format!("{:?}", baseline.cpu));
                assert_eq!(format!("{:?}", machine.bus), format!("{:?}", baseline.bus));
                assert!(machine.mem.data == baseline.mem.data, "RAM differs");
                assert_eq!(machine.cycles, baseline.cycles);
            };
            let start = Instant::now();
            let expected = run(&mut reference);
            let interpreter = start.elapsed();
            let native_start = Instant::now();
            let actual = run(&mut native);
            let elapsed = native_start.elapsed();
            assert_eq!(actual, expected);
            compare(&native, &reference);
            let cold_cycles = native.backend.native_entries();
            native.cpu = rx82::cpu::Cpu::default();
            native.cpu.pc = 0x100;
            native.bus = rx82::bus::Bus::default();
            native.cycles = 0;
            native.mem.data.clone_from(&initial_memory);
            let warm_start = Instant::now();
            assert_eq!(run(&mut native), expected);
            let warm = warm_start.elapsed();
            compare(&native, &reference);
            println!(
                "{name} sample={sample} cycles={actual} interpreter_ns={} cold_jit_ns={} warm_jit_ns={} cold_native_cycles={cold_cycles}",
                interpreter.as_nanos(),
                elapsed.as_nanos(),
                warm.as_nanos()
            );
        }
    }
}

#[test]
fn regions_preserve_the_preceding_store_commit() {
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    native.select_backend("jit").expect("JIT");
    // LD A,42; STORE $2000,A; NOP; HALT.
    for system in [&mut reference, &mut native] {
        system.cpu.pc = 0x100;
        system
            .mem
            .load(0x100, &[0x10, 42, 0x20, 0, 0x20, 1, 0])
            .expect("code");
    }
    assert_eq!(native.run_cycles(100), reference.run_cycles(100));
    assert_eq!(native.mem.get(0x2000), 42);
    assert!(native.mem.data == reference.mem.data, "RAM differs");
    assert_eq!(format!("{:?}", native.bus), format!("{:?}", reference.bus));
}

#[test]
fn pure_instruction_regions_preserve_state_at_all_short_budgets() {
    use r8cpu::instructions::{InstructionKind, Operands};
    for opcode in 0_u8..=255 {
        let pure = opcode <= 6 && opcode != 2
            || (0x10..=0x1C).contains(&opcode)
            || (0x30..=0x3C).contains(&opcode)
            || (0x40..=0x4C).contains(&opcode)
            || (0x50..=0xBC).contains(&opcode) && opcode & 15 <= 12
            || (0xF0..=0xF7).contains(&opcode);
        if !pure {
            continue;
        }
        let instruction = InstructionKind::try_from(opcode).expect("valid pure opcode");
        let mut program = vec![opcode];
        match instruction.operands() {
            Operands::Zero => (),
            Operands::One => program.push(if opcode >= 0xF0 { 0 } else { 0x81 }),
            Operands::Two => program.extend_from_slice(if opcode == 0xF7 {
                &[3, 1]
            } else {
                &[0x81, 0xFE]
            }),
        }
        program.push(0);
        for flags in [0_u8, 0xC7] {
            for budget in 0_u32..=16 {
                let mut reference = System {
                    turbo: true,
                    ..System::default()
                };
                let mut native = System {
                    turbo: true,
                    ..System::default()
                };
                native.select_backend("jit").expect("JIT");
                for system in [&mut reference, &mut native] {
                    system.cpu.pc = 0x100;
                    system.cpu.flags = flags.into();
                    system
                        .cpu
                        .regs
                        .set_bytes([0, 0xFF, 0x7F, 0x80, 1, 0xFE, 0xAA, 0x55]);
                    system.mem.load(0x100, &program).expect("code");
                }
                assert_eq!(
                    native.run_cycles(budget),
                    reference.run_cycles(budget),
                    "opcode {opcode:02X} budget {budget}"
                );
                assert_eq!(
                    format!("{:?}", native.cpu),
                    format!("{:?}", reference.cpu),
                    "opcode {opcode:02X} flags {flags:02X} budget {budget}"
                );
                assert_eq!(
                    format!("{:?}", native.bus),
                    format!("{:?}", reference.bus),
                    "opcode {opcode:02X} budget {budget}"
                );
            }
        }
    }
}

#[test]
fn native_ram_load_store_and_following_fetch_stay_in_one_region() {
    for budget in 0_u32..=40 {
        let mut reference = System {
            turbo: true,
            ..System::default()
        };
        let mut native = System {
            turbo: true,
            ..System::default()
        };
        native.select_backend("jit").expect("JIT");
        for system in [&mut reference, &mut native] {
            system.cpu.pc = 0x100;
            system.mem.set(0x1000, 0xA5);
            system
                .mem
                .load(0x100, &[0x19, 0, 0x10, 0x1D, 0x90, 0x20, 0, 0x20, 0x39, 0])
                .expect("code");
        }
        let expected = reference.run_cycles(budget);
        assert_eq!(native.run_cycles(budget), expected);
        assert_eq!(
            format!("{:?}", native.cpu),
            format!("{:?}", reference.cpu),
            "budget {budget}"
        );
        assert_eq!(
            format!("{:?}", native.bus),
            format!("{:?}", reference.bus),
            "budget {budget}"
        );
        assert!(native.mem.data == reference.mem.data, "RAM budget {budget}");
        if budget == 40 && !cfg!(feature = "states") {
            assert_eq!(expected, 32);
            assert_eq!(native.mem.get(0x2000), 0xA5);
            assert_eq!(
                native.backend.native_entries(),
                u64::from(expected),
                "RAM accesses and fetch remain native"
            );
        }
    }
}

#[test]
fn self_modifying_stores_and_rom_guards_preserve_every_boundary() {
    let programs: &[&[u8]] = &[
        &[
            0x19, 0xFF, 0xFF, 0x10, 0, 0x20, 0x0B, 1, 0x39, 1, 1, 0x30, 0,
        ],
        &[0x19, 0, 0xC0, 0x39, 0x1D, 0x90, 0],
        &[0x19, 0xFF, 0xBF, 0x39, 0x28, 0x09, 0],
        &[0x19, 0xFF, 0xFF, 0x1F, 0x90, 1, 0],
        &[0x19, 0, 0x10, 0x1F, 0x90, 255, 0x2F, 0x09, 254, 0],
        &[0x1C, 0, 0x20, 0x29, 12, 0x55, 0x39, 0],
    ];
    for program in programs {
        compare_program_budgets(program, 64);
    }
}

#[test]
fn resident_pairs_obey_byte_aliases_and_all_exit_paths() {
    let programs: &[&[u8]] = &[
        &[0x19, 0xFF, 0xFF, 0x39, 0x12, 0x80, 0x39, 0],
        &[0x18, 0xFF, 0x7F, 0x38, 0x10, 0x80, 0x38, 0],
        &[0x19, 1, 0, 0x39, 3, 0x39, 0x49, 0],
        &[0x19, 1, 0, 0x39, 0x1A, 0xFF, 0xFF, 0x3A, 0x71, 3, 0],
        &[
            0x18, 1, 0, 0x19, 2, 0, 0x1A, 3, 0, 0x1B, 4, 0, 0x1C, 0, 0x10, 0x38, 0x39, 0x3A, 0x3B,
            0x3C, 0,
        ],
        &[0x19, 2, 0, 0xF0, 1, 0x39, 0x49, 0xF2, 0xFD, 0],
    ];
    for program in programs {
        compare_program_budgets(program, 120);
    }
}

fn compare_program_budgets(program: &[u8], maximum: u32) {
    for budget in 0..=maximum {
        let mut reference = System {
            turbo: true,
            ..System::default()
        };
        let mut native = System {
            turbo: true,
            ..System::default()
        };
        native.select_backend("jit").expect("JIT");
        for system in [&mut reference, &mut native] {
            system.cpu.pc = 0x100;
            system.cpu.flags = 0xC7.into();
            system.mem.set(0x1000, 0xA5);
            system.mem.set(0x10FF, 0xC3);
            system.mem.load(0x100, program).expect("code");
        }
        assert_eq!(native.run_cycles(budget), reference.run_cycles(budget));
        assert_eq!(
            format!("{:?}", native.cpu),
            format!("{:?}", reference.cpu),
            "program {program:02X?} budget {budget}"
        );
        assert_eq!(
            format!("{:?}", native.bus),
            format!("{:?}", reference.bus),
            "program {program:02X?} budget {budget}"
        );
        assert!(
            native.mem.data == reference.mem.data,
            "RAM program {program:02X?} budget {budget}"
        );
        assert_eq!(native.cycles, reference.cycles);
    }
}

#[test]
fn fused_count_loops_preserve_wrapping_flags_and_partial_budgets() {
    for opcode in [
        0x38_u8, 0x39, 0x3A, 0x3B, 0x3C, 0x48, 0x49, 0x4A, 0x4B, 0x4C,
    ] {
        for value in [0_u16, 1, 2, 0x7FFF, 0xFFFF] {
            let [lo, hi] = value.to_le_bytes();
            let load = 0x10 | (opcode & 15);
            compare_program_budgets(&[load, lo, hi, opcode, 0xF2, 0xFD, 0], 70);
        }
    }
}
