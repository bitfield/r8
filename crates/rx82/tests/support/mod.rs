// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    clippy::definition_in_module_root,
    reason = "shared integration test support"
)]
#![expect(
    clippy::arbitrary_source_item_ordering,
    reason = "snapshot fields follow capture order"
)]
#![expect(
    clippy::map_with_unused_argument_over_ranges,
    reason = "the cycle count bounds execution"
)]

use rx82::{state::State, system::System};

/// CPU, bus, RAM and cycle count after one scheduler tick.
/// The bus snapshot includes pending updates.
#[derive(Debug, PartialEq)]
pub struct Cycle {
    pub state: State,
    pub cpu: String,
    pub bus: String,
    pub ram: Vec<u8>,
    pub ram_start: u16,
    pub ram_end: u16,
    pub cycles: u16,
}

/// Executes exactly the requested ticks, even for a halted or intermediate CPU.
/// State is separate because its debug representation omits payloads.
pub fn run_bounded(system: &mut System, limit: usize) -> Vec<Cycle> {
    (0..limit)
        .map(|_| {
            system.tick();
            Cycle {
                state: system.cpu.state,
                cpu: format!("{:?}", system.cpu),
                bus: format!("{:?}", system.bus),
                ram: system.mem.data.clone(),
                ram_start: system.mem.start,
                ram_end: system.mem.end,
                cycles: system.cycles,
            }
        })
        .collect()
}
