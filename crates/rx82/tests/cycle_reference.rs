// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![expect(
    clippy::pattern_type_mismatch,
    reason = "borrow trace entries with slice patterns"
)]
#![expect(clippy::panic, reason = "assert exact trace lengths in tests")]
#![expect(clippy::std_instead_of_alloc, reason = "test harness uses std")]

mod support;

use core::cell::RefCell;
use rx82::{state::State, system::System};
use std::rc::Rc;

use rx82::{bus::Bus, system::Device};

struct Observer(Rc<RefCell<Vec<(u16, u8, bool)>>>);

impl Device for Observer {
    fn tick(&mut self, bus: &mut Bus) {
        self.0.borrow_mut().push((bus.addr, bus.data, bus.mem));
    }
}

#[test]
fn cycle_limit_stops_a_running_machine_without_resetting_it() {
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system.cpu.pc = 0x0100;
    let trace = support::run_bounded(&mut system, 1);
    assert_eq!(trace.len(), 1);
    assert_eq!(system.cycles, 1);
    assert_eq!(system.cpu.pc, 0x0101);
    assert_eq!(system.cpu.state, State::WaitOpcode);
}

#[test]
fn zero_budget_preserves_initial_state() {
    let mut system = System::default();
    system.cpu.state = State::PushData(0x42);
    assert_eq!(support::run_bounded(&mut system, 0).len(), 0);
    assert_eq!(system.cpu.state, State::PushData(0x42));
    assert_eq!(system.cycles, 0);
}

#[test]
fn traces_distinguish_continuation_payloads() {
    let mut first = System {
        turbo: true,
        ..System::default()
    };
    let mut second = System {
        turbo: true,
        ..System::default()
    };
    first.cpu.state = State::WaitData(r8cpu::regs::Reg::A);
    second.cpu.state = State::WaitData(r8cpu::regs::Reg::B);
    assert_ne!(
        support::run_bounded(&mut first, 1),
        support::run_bounded(&mut second, 1)
    );
}

#[test]
fn trace_records_literal_fetch_timing_and_device_observations() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut system = System {
        turbo: true,
        ..System::default()
    };
    system
        .devices
        .push(Box::new(Observer(Rc::clone(&observations))));
    system.cpu.pc = 0x0100;
    system.mem.set(0x0100, 0x01); // NOP
    let trace = support::run_bounded(&mut system, 2);
    let [first, second] = trace.as_slice() else {
        panic!("expected exactly two cycles");
    };
    assert_eq!(*observations.borrow(), [(0, 0, false), (0x0100, 0, true)]);
    assert_eq!(first.cycles, 1);
    assert_eq!(second.cycles, 2);
    assert_ne!(first.cpu, second.cpu);
    assert_ne!(first.bus, second.bus);
    assert_eq!(second.ram.get(0x0100), Some(&0x01));
    assert_eq!((second.ram_start, second.ram_end), (0, 0xBFFF));
    assert_eq!(system.cpu.state, State::Decode);
}

#[test]
fn halted_cpu_still_ticks_devices_and_wraps_the_cycle_counter() {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let mut system = System {
        cycles: u16::MAX,
        turbo: true,
        ..System::default()
    };
    system.cpu.halt = true;
    system
        .devices
        .push(Box::new(Observer(Rc::clone(&observations))));
    let trace = support::run_bounded(&mut system, 1);
    let [cycle] = trace.as_slice() else {
        panic!("expected exactly one cycle");
    };
    assert_eq!(cycle.cycles, 0);
    assert_eq!(system.cpu.state, State::FetchOpcode);
    assert_eq!(observations.borrow().len(), 1);
}
