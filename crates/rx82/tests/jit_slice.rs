// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(clippy::std_instead_of_alloc, reason = "test harness uses std")]
#![expect(clippy::expect_used, reason = "test setup and assertions")]

mod support;

use core::cell::Cell;
use r8cpu::instructions::InstructionKind;
use r8cpu::regs::Reg;
use rx82::state::State;
use rx82::system::System;
use rx82::{bus::Bus, system::Device};
use std::rc::Rc;

struct OperandDevice(Rc<Cell<u32>>);

impl Device for OperandDevice {
    fn tick(&mut self, bus: &mut Bus) {
        self.0.set(self.0.get().wrapping_add(1));
        if bus.mem && !bus.write && bus.addr == 0x0101 {
            bus.write_data(2);
        }
    }
}

#[test]
fn public_instruction_variant_with_an_aliased_opcode_falls_back() {
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
        system.cpu.regs.set16(Reg::SP, 0x0300);
        system.mem.set(0x0100, 0xDC);
        for _ in 0_u8..3 {
            system.tick();
        }
        system.cpu.ins = InstructionKind::Push(Reg::SP);
        system.tick();
    }
    assert_eq!(reference.cpu.state, State::PushData(3), "edited word push");
    assert_eq!(
        native.cpu.state, reference.cpu.state,
        "instruction identity includes register payload"
    );
    assert_eq!(native.bus.data, reference.bus.data, "edited push data");
    assert_eq!(
        native.backend.native_entries(),
        0,
        "aliased opcode selection discarded"
    );
}

#[test]
fn compilation_uses_device_supplied_operand_without_extra_ticks() {
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    let reference_ticks = Rc::new(Cell::new(0));
    let native_ticks = Rc::new(Cell::new(0));
    reference
        .devices
        .push(Box::new(OperandDevice(Rc::clone(&reference_ticks))));
    native
        .devices
        .push(Box::new(OperandDevice(Rc::clone(&native_ticks))));
    for system in [&mut reference, &mut native] {
        system.cpu.pc = 0x0100;
        system
            .mem
            .load(0x0100, &[0x10, 1, 0])
            .expect("program fits");
    }
    native
        .select_backend("jit")
        .expect("native backend available");
    assert_eq!(
        support::run_bounded(&mut native, 10),
        support::run_bounded(&mut reference, 10)
    );
    assert_eq!(native.cpu.regs.get(Reg::A), 2);
    assert_eq!(native.mem.get(0x0101), 1);
    assert_eq!(native.backend.native_entries(), 2);
    assert_eq!(reference_ticks.get(), 10);
    assert_eq!(native_ticks.get(), 10);
}

#[test]
fn changed_instruction_bytes_select_new_native_code() {
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
        system
            .mem
            .load(0x0100, &[0x10, 1, 0xF0, 0xFC])
            .expect("program fits");
        support::run_bounded(system, 6);
        system.mem.set(0x0101, 2);
    }
    assert_eq!(
        support::run_bounded(&mut native, 12),
        support::run_bounded(&mut reference, 12)
    );
    assert_eq!(native.cpu.regs.get(Reg::A), 2);
    assert_eq!(native.backend.native_entries(), 3);
}

#[test]
fn native_stack_request_preserves_an_existing_bus_writer() {
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
        system.cpu.regs.set16(Reg::SP, 0x0200);
        system
            .mem
            .load(0x0100, &[0xF8, 0x10, 0x01])
            .expect("program fits");
        support::run_bounded(system, 7);
        system.bus.read_mem(0x0300);
    }
    assert_eq!(
        support::run_bounded(&mut native, 1),
        support::run_bounded(&mut reference, 1)
    );
    assert_eq!(native.bus.addr, 0x0300);
    assert_eq!(native.cpu.regs.get16(Reg::SP), 0x01FF);
    assert_eq!(native.backend.native_entries(), 1);
}

#[test]
fn public_operand_and_instruction_edits_discard_specialisation() {
    for edit in 0_u8..3 {
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
            system.mem.load(0x0100, &[0x10, 1]).expect("program fits");
            support::run_bounded(system, 5);
            assert_eq!(system.cpu.state, State::Execute);
            match edit {
                0 => system.cpu.op_lo = 2,
                1 => system.cpu.op_hi = 2,
                _ => system.cpu.ins = InstructionKind::Inc(Reg::A),
            }
        }
        assert_eq!(
            support::run_bounded(&mut native, 1),
            support::run_bounded(&mut reference, 1)
        );
        assert_eq!(native.backend.native_entries(), 0);
        assert_eq!(native.cpu.regs.get(Reg::A), if edit == 0 { 2 } else { 1 });
    }
}

#[test]
fn continuation_edits_and_backend_switches_use_the_current_state_once() {
    for edit in 0_u8..3 {
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
            system.cpu.regs.set16(Reg::SP, 0x0200);
            system
                .mem
                .load(0x0100, &[0xF8, 0x10, 0x01])
                .expect("program fits");
            support::run_bounded(system, 8);
            assert_eq!(system.cpu.state, State::WaitCall(3, 0x0110));
            match edit {
                0 => system.cpu.state = State::WaitCall(9, 0x0130),
                1 => system.cpu.state = State::PushData(9),
                _ => (),
            }
        }
        if edit == 2 {
            native
                .select_backend("interpreter")
                .expect("interpreter available");
            native
                .select_backend("jit")
                .expect("native backend available");
        }
        assert_eq!(
            support::run_bounded(&mut native, 1),
            support::run_bounded(&mut reference, 1)
        );
        assert_eq!(native.backend.native_entries(), u64::from(edit != 2));
    }
}

#[test]
fn initial_execute_without_fetch_history_falls_back_once() {
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system.cpu.state = State::Execute;
    system.cpu.ins = InstructionKind::Ld(Reg::A);
    system.cpu.op_lo = 7;
    system
        .select_backend("jit")
        .expect("native backend available");
    support::run_bounded(&mut system, 1);
    assert_eq!(system.cpu.regs.get(Reg::A), 7);
    assert_eq!(system.cycles, 1);
    assert_eq!(system.backend.native_entries(), 0);
}

#[test]
fn arithmetic_branch_loop_executes_natively_with_literal_result() {
    let program = [0x10, 0xFE, 0x50, 1, 0xF2, 0xFC, 0];
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    for system in [&mut reference, &mut native] {
        system.cpu.pc = 0x0100;
        system.mem.load(0x0100, &program).expect("program fits");
    }
    native
        .select_backend("jit")
        .expect("native backend available");
    assert_eq!(
        support::run_bounded(&mut native, 34),
        support::run_bounded(&mut reference, 34)
    );
    assert_eq!(native.cpu.regs.get(Reg::A), 0);
    assert!(native.cpu.halt);
    assert_eq!(native.backend.native_entries(), 6);
}

#[test]
fn native_call_resumes_on_the_next_cycle() {
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    for system in [&mut reference, &mut native] {
        system.cpu.pc = 0x0100;
        system.cpu.regs.set16(Reg::SP, 0x0200);
        system
            .mem
            .load(0x0100, &[0xF8, 0x10, 0x01])
            .expect("program fits");
    }
    native
        .select_backend("jit")
        .expect("native backend available");
    assert_eq!(
        support::run_bounded(&mut native, 13),
        support::run_bounded(&mut reference, 13)
    );
    assert_eq!(native.mem.get(0x0200), 1);
    assert_eq!(native.mem.get(0x01FF), 3);
    assert_eq!(native.cpu.regs.get16(Reg::SP), 0x01FE);
    assert_eq!(native.backend.native_entries(), 3);
}

#[test]
fn native_nop_matches_every_reference_cycle() {
    let mut reference = System {
        turbo: true,
        ..System::default()
    };
    let mut native = System {
        turbo: true,
        ..System::default()
    };
    for system in [&mut reference, &mut native] {
        system.cpu.pc = 0x0100;
        system
            .mem
            .load(0x0100, &[0x01, 0x00])
            .expect("program fits");
    }
    native
        .select_backend("jit")
        .expect("native backend available");
    assert_eq!(
        support::run_bounded(&mut native, 8),
        support::run_bounded(&mut reference, 8)
    );
    assert_eq!(native.backend.native_entries(), 2);
}
