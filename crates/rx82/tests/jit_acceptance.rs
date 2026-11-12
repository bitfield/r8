// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "bounded acceptance assertions"
)]
#![expect(
    clippy::arbitrary_source_item_ordering,
    reason = "keep machine fields and acceptance fixtures together"
)]
#![expect(clippy::std_instead_of_alloc, reason = "test harness uses std")]
#![expect(
    clippy::default_numeric_fallback,
    reason = "opcode and shift literals in test data"
)]
#![expect(clippy::shadow_unrelated, reason = "independent clock fixtures")]

use r8cpu::regs::Reg;
use rx82::{cpu::Cpu, state::State, system::System};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq)]
struct Core {
    state: State,
    regs: [u8; 8],
    sp: u16,
    pc: u16,
    flags: u8,
    halt: bool,
    ins: u8,
    operands: [u8; 2],
    id: ([u8; 8], [u8; 6]),
}

fn core(cpu: &Cpu) -> Core {
    Core {
        state: cpu.state,
        regs: [
            Reg::A,
            Reg::B,
            Reg::C,
            Reg::D,
            Reg::E,
            Reg::F,
            Reg::G,
            Reg::H,
        ]
        .map(|reg| cpu.regs.get(reg)),
        sp: cpu.regs.get16(Reg::SP),
        pc: cpu.pc,
        flags: u8::from(cpu.flags),
        halt: cpu.halt,
        ins: u8::from(cpu.ins),
        operands: [cpu.op_lo, cpu.op_hi],
        id: (
            *cpu.id.vendor,
            [
                cpu.id.highest,
                cpu.id.extended,
                cpu.id.family,
                cpu.id.model,
                cpu.id.revision,
                u8::from(cpu.id.capabilities),
            ],
        ),
    }
}

fn compare(reference: &System, native: &System, label: &str) {
    assert_eq!(core(&native.cpu), core(&reference.cpu), "{label} CPU");
    assert_eq!(
        (
            native.bus.addr,
            native.bus.data,
            native.bus.mem,
            native.bus.write,
            native.bus.debug
        ),
        (
            reference.bus.addr,
            reference.bus.data,
            reference.bus.mem,
            reference.bus.write,
            reference.bus.debug
        ),
        "{label} bus"
    );
    if native.bus.pending_write.is_some() || reference.bus.pending_write.is_some() {
        assert_eq!(
            format!("{:?}", native.bus.pending_write),
            format!("{:?}", reference.bus.pending_write),
            "{label} arbitration"
        );
    }
    assert_eq!(native.cycles, reference.cycles, "{label} cycles");
    assert_eq!(
        (native.mem.start, native.mem.end),
        (reference.mem.start, reference.mem.end),
        "{label} RAM map"
    );
}

fn compare_ram(reference: &System, native: &System, label: &str) {
    assert_eq!(
        native.mem.data.len(),
        reference.mem.data.len(),
        "{label} RAM size"
    );
    if let Some(index) = native
        .mem
        .data
        .iter()
        .zip(&reference.mem.data)
        .position(|(actual, expected)| actual != expected)
    {
        panic!("{label} RAM differs at {index:04X}");
    }
}

fn tick(reference: &mut System, native: &mut System, label: &str) {
    let writes = (reference.bus.mem && reference.bus.write) || (native.bus.mem && native.bus.write);
    reference.tick();
    native.tick();
    compare(reference, native, label);
    if writes {
        compare_ram(reference, native, label);
    }
}

fn pair(program: &[u8]) -> (System, System) {
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
    for machine in [&mut reference, &mut native] {
        machine.cpu.pc = 0x0100;
        machine.cpu.regs.set16(Reg::SP, 0x0300);
        machine.cpu.regs.set16(Reg::AB, 0x0200);
        machine.mem.load(0x0100, program).expect("program fits");
    }
    (reference, native)
}

#[test]
fn switching_backends_at_every_state_preserves_the_next_cycles() {
    let programs: &[&[u8]] = &[
        &[1],
        &[0x10, 1],
        &[0x18, 1, 2],
        &[0x1D, 0x80],
        &[0x3E, 0, 2],
        &[0x4E, 0, 2],
        &[0xD8],
        &[0xE8],
        &[0xEC],
        &[7],
        &[8],
        &[0xF8, 0, 2],
        &[0xF9, 1],
    ];
    let mut seen = BTreeSet::new();
    for program in programs {
        for checkpoint in 0_u8..16 {
            let (mut reference, mut native) = pair(program);
            for _ in 0..checkpoint {
                tick(&mut reference, &mut native, "before switch");
            }
            seen.insert(reference.cpu.state.to_string());
            native
                .select_backend("interpreter")
                .expect("interpreter available");
            tick(&mut reference, &mut native, "interpreter after switch");
            native
                .select_backend("jit")
                .expect("native backend available");
            for _ in 0_u8..32 {
                tick(&mut reference, &mut native, "resumed after switch");
            }
            compare_ram(&reference, &native, "after switch");
        }
    }
    assert_eq!(
        seen.len(),
        34,
        "every state variant must be exercised: {seen:?}"
    );
}

#[test]
fn wraparound_rom_writes_and_stack_boundaries_preserve_cycles() {
    let programs: &[&[u8]] = &[
        &[0xD8],
        &[0xE8],
        &[0xF8, 0, 2],
        &[0xF9, 1],
        &[7],
        &[8],
        &[0x1F, 0x80, 0xFF],
        &[0x2F, 0x08, 0xFF],
        &[0x20, 0, 0xC0],
        &[0x3E, 0xFF, 0xFF],
    ];
    for program in programs {
        for stack in [0, 1, 0xBFFF, 0xFFFF] {
            let (mut reference, mut native) = pair(program);
            for machine in [&mut reference, &mut native] {
                machine.cpu.regs.set16(Reg::SP, stack);
                machine.cpu.regs.set16(Reg::AB, 0xFFFF);
            }
            for _ in 0_u8..16 {
                tick(&mut reference, &mut native, "boundary");
            }
            compare_ram(&reference, &native, "boundary");
        }
    }
    for opcode in 0xF0_u8..=0xF6 {
        for pc in [0, 1, 0xFFFE, 0xFFFF] {
            let (mut reference, mut native) = pair(&[opcode, 0x80]);
            for _ in 0_u8..5 {
                tick(&mut reference, &mut native, "branch fetch");
            }
            reference.cpu.pc = pc;
            native.cpu.pc = pc;
            tick(&mut reference, &mut native, "branch wrap");
            assert_eq!(native.backend.native_entries(), 1, "branch is native");
        }
    }
}

#[test]
fn deterministic_initial_state_fuzz_preserves_native_instruction_cycles() {
    use r8cpu::instructions::{InstructionKind, Operands};
    let opcodes: Vec<u8> = (0_u8..=255)
        .filter(|opcode| InstructionKind::try_from(*opcode).is_ok())
        .collect();
    let mut random = 0x8D31_746B_u32;
    let next = |state: &mut u32| {
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        state.to_be_bytes()[0]
    };
    for case in 0_u16..2048 {
        let seed = random;
        let opcode = *opcodes
            .get(
                usize::from(next(&mut random))
                    .checked_rem(opcodes.len())
                    .expect("nonempty opcode table"),
            )
            .expect("valid opcode");
        let mut bytes = [opcode, next(&mut random), next(&mut random)];
        if matches!(opcode, 0x6F | 0x8F | 0x9F | 0xAF | 0xBF) {
            let source = bytes[1] >> 4;
            let target = bytes[1] & 15;
            if source < 13 && target < 13 && (source < 8) != (target < 8) {
                bytes[1] = if target < 8 {
                    ((source & 7) << 4) | target
                } else {
                    ((8 + source.rem_euclid(5)) << 4) | target
                };
            }
        }
        if opcode == 0xF9 && bytes[1] == 0x20 {
            bytes[1] = 0x21;
        }
        let length = match InstructionKind::try_from(opcode)
            .expect("valid opcode")
            .operands()
        {
            Operands::Zero => 1,
            Operands::One => 2,
            Operands::Two => 3,
        };
        let (mut reference, mut native) = pair(&[]);
        let pc = match case & 7 {
            0 => 0xFFFD,
            1 => 0xFFFE,
            2 => 0xFFFF,
            _ => u16::from_be_bytes([next(&mut random), next(&mut random)]),
        };
        let stack = match case & 7 {
            0 => 0,
            1 => 1,
            2 => 0xFFFF,
            _ => u16::from_be_bytes([next(&mut random), next(&mut random)]),
        };
        let registers = [0_u8; 8].map(|_| next(&mut random));
        let flags = next(&mut random);
        for machine in [&mut reference, &mut native] {
            machine.devices.clear();
            machine.mem.end = 0xFFFF;
            machine.mem.data.resize(0x0001_0000, 0);
            machine.cpu.pc = pc;
            machine.cpu.flags = flags.into();
            machine.cpu.regs.set16(Reg::SP, stack);
            machine.cpu.id.vendor = b"R8JITXYZ";
            machine.cpu.id.capabilities.emulation = false;
            machine.cpu.id.family = flags;
            machine.cpu.id.model = registers[0];
            machine.cpu.id.highest = registers[1];
            machine.cpu.id.revision = registers[2];
            machine.cpu.id.extended = registers[3];
            for (reg, value) in (0_u8..8).zip(registers) {
                machine
                    .cpu
                    .regs
                    .set(Reg::try_from(reg).expect("byte register"), value);
            }
            for (offset, value) in bytes.iter().take(length).enumerate() {
                machine.mem.set(
                    pc.wrapping_add(u16::try_from(offset).expect("instruction offset")),
                    *value,
                );
            }
        }
        let label = format!("seed={seed:08X} case={case} pc={pc:04X} code={bytes:02X?}");
        let mut executing = false;
        let mut native_cycles = 0_u64;
        let mut completed = false;
        for _ in 0_u8..32 {
            executing |= reference.cpu.state == State::Execute;
            tick(&mut reference, &mut native, &label);
            if executing {
                native_cycles = native_cycles.wrapping_add(1);
            }
            if executing && reference.cpu.state == State::FetchOpcode {
                completed = true;
                break;
            }
        }
        assert!(completed, "{label} instruction completed");
        assert_eq!(
            native.backend.native_entries(),
            native_cycles,
            "{label} native witness"
        );
        compare_ram(&reference, &native, &label);
    }
}

#[test]
fn normal_clock_pending_requests_and_turbo_skips_match() {
    use core::cell::Cell;
    use rx82::{bus::Bus, system::Device};
    use std::rc::Rc;
    struct Clock(Rc<Cell<u32>>);
    impl Device for Clock {
        fn tick(&mut self, bus: &mut Bus) {
            self.0.set(self.0.get().wrapping_add(1));
            bus.write_data(0xA5);
        }
    }
    for turbo in [false, true] {
        let (mut reference, mut native) = pair(&[0x10, 1, 0x50, 1, 0]);
        let ref_ticks = Rc::new(Cell::new(0));
        let jit_ticks = Rc::new(Cell::new(0));
        reference.clock = Box::new(Clock(Rc::clone(&ref_ticks)));
        native.clock = Box::new(Clock(Rc::clone(&jit_ticks)));
        reference.turbo = turbo;
        native.turbo = turbo;
        for _ in 0_u8..24 {
            tick(&mut reference, &mut native, "clock arbitration");
        }
        assert_eq!(
            ref_ticks.get(),
            if turbo { 0 } else { 24 },
            "clock tick count"
        );
        assert_eq!(jit_ticks.get(), ref_ticks.get(), "native clock tick count");
        compare_ram(&reference, &native, "clock arbitration");
    }
    let (mut reference, mut native) = pair(&[0x10, 1, 0]);
    reference.turbo = false;
    native.turbo = false;
    for _ in 0_u8..10 {
        tick(&mut reference, &mut native, "default normal clock");
    }
    assert_eq!(native.cpu.regs.get(Reg::A), 1, "normal clock result");
}

#[test]
fn debug_history_preserves_start_state_and_reconciled_bus() {
    let (mut reference, mut native) = pair(&[0x18, 1, 2, 0xD8, 0xE8, 0xF9, 1, 0]);
    reference.debug = true;
    native.debug = true;
    for _ in 0_u8..40 {
        tick(&mut reference, &mut native, "debug snapshot");
    }
    assert_eq!(native.history.len(), 40, "one snapshot per cycle");
    for (actual, expected) in native.history.iter().zip(&reference.history) {
        assert_eq!(actual.state, expected.state, "snapshot start state");
        assert_eq!(actual.tick, expected.tick, "snapshot counter");
        assert_eq!(
            format!("{:?}", actual.bus),
            format!("{:?}", expected.bus),
            "snapshot reconciled bus"
        );
    }
}

#[test]
#[ignore = "manual full compatibility corpus in release mode"]
fn rom_boot_and_supplied_programs_match_each_cycle() {
    compatibility_corpus(false);
}

#[test]
#[ignore = "manual bounded compatibility corpus in release mode"]
fn bounded_rom_and_supplied_programs_match() {
    compatibility_corpus(true);
}

fn compatibility_corpus(bounded: bool) {
    let (mut reference, mut native) = pair(&[]);
    for machine in [&mut reference, &mut native] {
        machine.cpu.reset(&mut machine.bus);
    }
    let run = |reference: &mut System, native: &mut System, limit: u32, label: &str| {
        if bounded {
            let mut cycles = 0_u32;
            while cycles < limit {
                let budget = limit.saturating_sub(cycles).min(4093);
                let expected = reference.run_cycles(budget);
                assert_eq!(
                    native.run_cycles(budget),
                    expected,
                    "{label} retired cycles"
                );
                cycles = cycles.wrapping_add(expected);
                compare(reference, native, label);
                compare_ram(reference, native, label);
                if reference.cpu.halt {
                    return;
                }
            }
            panic!("{label} exceeded {limit} cycles");
        }
        for _ in 0..limit {
            tick(reference, native, label);
            if reference.cpu.halt && reference.cpu.state == State::FetchOpcode {
                compare_ram(reference, native, label);
                return;
            }
        }
        panic!("{label} exceeded {limit} cycles");
    };
    run(&mut reference, &mut native, 10_000_000, "ROM boot");
    assert_eq!(native.cpu.regs.get16(Reg::SP), 0xBFFF, "ROM found RAM top");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["baseline", "clc", "sec"] {
        let program =
            r8asm::assemble_source_file(root.join(format!("../r8cpu/functional_tests/{name}.asm")))
                .expect("assemble functional program");
        for machine in [&mut reference, &mut native] {
            machine.mem.load(0x0100, &program).expect("program fits");
            machine.cpu.pc = 0x0100;
            machine.cpu.halt = false;
        }
        run(&mut reference, &mut native, 100_000, name);
        assert_eq!(
            native.cpu.regs.get(Reg::A),
            b'.',
            "functional program passed"
        );
    }
    for name in ["countdown", "memcpy", "ram_test", "to_hex"] {
        let program =
            r8asm::assemble_source_file(root.join(format!("../r8asm/examples/{name}.asm")))
                .expect("assemble supplied example");
        for machine in [&mut reference, &mut native] {
            machine.mem.load(0x0100, &program).expect("program fits");
            machine.cpu.pc = 0x0100;
            machine.cpu.halt = false;
        }
        run(&mut reference, &mut native, 15_000_000, name);
    }
    assert!(
        native.backend.native_entries() > 1_000_000,
        "corpus executed natively"
    );
}
