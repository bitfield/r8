// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(clippy::single_call_fn, reason = "keep region lifecycle separate")]
#![expect(clippy::semicolon_outside_block, reason = "native call is a statement")]
use super::{context, ffi};
use crate::{bus::Bus, cpu::Cpu, memory::Memory};
use anyhow::{Result, bail};
use std::collections::HashMap;

struct Region {
    last_used: u64,
    source: Vec<u8>,
    entry: usize,
    allocated: u64,
}

pub(super) struct Regions {
    clock: u64,
    handle: u64,
    entries: HashMap<u16, Region>,
    allocated: u64,
}

impl Regions {
    pub(super) fn cache_usage(&self) -> (usize, u64) {
        (
            self.entries
                .values()
                .filter(|entry| entry.entry != 0)
                .count(),
            self.allocated,
        )
    }

    fn evict(&mut self) -> bool {
        let key = self
            .entries
            .iter()
            .min_by_key(|entry| (entry.1.last_used, *entry.0))
            .map(|entry| *entry.0);
        key.is_some_and(|key| self.remove_entry(key))
    }

    pub(super) fn new() -> Result<Self> {
        let mut handle = 0;
        // SAFETY: output lives through the synchronous call.
        if unsafe {
            ffi::r8jit_create(
                ffi::R8JIT_ABI_VERSION,
                1024 * 1024,
                &raw mut handle,
                core::ptr::null_mut(),
                0,
            )
        } != 0_i32
        {
            bail!("region compiler creation failed");
        }
        Ok(Self {
            clock: 0,
            handle,
            entries: HashMap::new(),
            allocated: 0,
        })
    }

    fn remove_entry(&mut self, key: u16) -> bool {
        let Some(old) = self.entries.get(&key) else {
            return false;
        };
        // SAFETY: execution is stopped and this cache exclusively owns the entry.
        if old.entry != 0 && unsafe { ffi::r8jit_release(self.handle, old.entry) } != 0_i32 {
            return false;
        }
        self.allocated = self.allocated.saturating_sub(old.allocated);
        self.entries.remove(&key);
        true
    }

    pub(super) fn run(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut Bus,
        source: &[u8],
        budget: u32,
        ram: Option<&mut Memory>,
    ) -> u32 {
        if budget < 4 {
            return 0;
        }
        self.clock = self.clock.saturating_add(1);
        let pc = cpu.pc;
        if let Some(old) = self.entries.get(&pc)
            && source.get(..old.source.len()) != Some(old.source.as_slice())
            && !self.remove_entry(pc)
        {
            return 0;
        }
        if !self.entries.contains_key(&pc) {
            if self.entries.len() >= 64 && !self.evict() {
                return 0;
            }
            let mut entry = 0;
            let mut allocated = 0;
            let mut consumed = 0;
            let status = loop {
                // SAFETY: compilation copies source synchronously and retains no
                // Rust pointers. The C declaration uses a mutable input pointer.
                let status = unsafe {
                    ffi::r8jit_compile_region(
                        self.handle,
                        pc,
                        source.as_ptr().cast_mut(),
                        u32::try_from(source.len()).unwrap_or_default(),
                        &raw mut entry,
                        &raw mut allocated,
                        &raw mut consumed,
                    )
                };
                if status == i32::try_from(ffi::R8JIT_BUDGET).unwrap_or_default() && self.evict() {
                    continue;
                }
                break status;
            };
            if status != 0_i32 {
                if status == i32::try_from(ffi::R8JIT_INSTRUCTION).unwrap_or_default() {
                    self.entries.insert(
                        pc,
                        Region {
                            last_used: self.clock,
                            source: source.iter().take(3).copied().collect(),
                            entry: 0,
                            allocated: 0,
                        },
                    );
                }
                return 0;
            }
            let Some(bytes) = source.get(..usize::try_from(consumed).unwrap_or_default()) else {
                return 0;
            };
            self.entries.insert(
                pc,
                Region {
                    last_used: self.clock,
                    source: bytes.to_vec(),
                    entry,
                    allocated,
                },
            );
            self.allocated = self.allocated.wrapping_add(allocated);
        }
        let Some(region) = self.entries.get_mut(&pc) else {
            return 0;
        };
        region.last_used = self.clock;
        if region.entry == 0 {
            return 0;
        }
        let (pointer, start, length) = ram.map_or((core::ptr::null_mut(), 0, 0), |memory| {
            let length = if memory.start <= memory.end {
                usize::from(memory.end.saturating_sub(memory.start))
                    .saturating_add(1)
                    .min(memory.data.len())
            } else {
                0
            };
            (
                memory.data.as_mut_ptr(),
                memory.start,
                u32::try_from(length).unwrap_or_default(),
            )
        });
        let mut state = ffi::r8_region {
            ram: pointer,
            ram_start: start,
            ram_length: length,
            cpu: context::encode(cpu, bus),
            budget,
            ..Default::default()
        };
        // SAFETY: the compiler owns live RX code with the C region signature.
        let run: unsafe extern "C" fn(*mut ffi::r8_region) =
            unsafe { core::mem::transmute(region.entry) };
        // SAFETY: native code accesses this stack context and the exclusively
        // borrowed RAM slice within its checked bounds. There are no callbacks.
        unsafe {
            run(&raw mut state);
        }
        if state.cycles == 0 {
            return 0;
        }
        if !context::apply(&state.cpu, cpu, bus) {
            return 0;
        }
        if let Ok(instruction) = r8cpu::instructions::InstructionKind::try_from(state.cpu.ins) {
            cpu.ins = instruction;
        }
        bus.addr = state.bus_addr;
        bus.data = state.cpu.bus_data;
        bus.mem = state.bus_mem != 0;
        bus.write = state.bus_write != 0;
        state.cycles
    }
}

impl Drop for Regions {
    fn drop(&mut self) {
        // SAFETY: exclusive ownership and no native call remains active.
        unsafe {
            ffi::r8jit_destroy(self.handle, core::ptr::null_mut(), 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[expect(clippy::expect_used, reason = "small compiler budget fixture")]
    fn byte_budget_evicts_cold_code_before_native_coverage_is_lost() {
        let mut cache = Regions::new().expect("compiler");
        assert_eq!(
            // SAFETY: this fresh compiler has no published entries.
            unsafe { ffi::r8jit_destroy(cache.handle, core::ptr::null_mut(), 0) },
            0_i32
        );
        assert_eq!(
            // SAFETY: output is a live handle field and no execution is active.
            unsafe {
                ffi::r8jit_create(
                    ffi::R8JIT_ABI_VERSION,
                    8192,
                    &raw mut cache.handle,
                    core::ptr::null_mut(),
                    0,
                )
            },
            0_i32
        );
        let mut cpu = Cpu::default();
        let mut bus = Bus::default();
        for pc in [0x100_u16, 0x200, 0x300] {
            cpu.pc = pc;
            assert_eq!(
                cache.run(&mut cpu, &mut bus, &[1, 0], 4, None),
                4,
                "new region must execute natively"
            );
        }
        assert_eq!(cache.cache_usage(), (2, 8192));
    }

    #[test]
    #[expect(clippy::expect_used, reason = "compiler fixture")]
    fn cache_pressure_retains_the_recently_used_region() {
        let mut cache = Regions::new().expect("compiler");
        let mut cpu = Cpu::default();
        let mut bus = Bus::default();
        for pc in 0x100_u16..0x140 {
            cpu.pc = pc;
            assert_eq!(cache.run(&mut cpu, &mut bus, &[2, 0, 0], 4, None), 0);
        }
        cpu.pc = 0x100;
        cache.run(&mut cpu, &mut bus, &[2, 0, 0], 4, None);
        cpu.pc = 0x200;
        cache.run(&mut cpu, &mut bus, &[2, 0, 0], 4, None);
        assert_eq!(cache.entries.len(), 64);
        assert!(cache.entries.contains_key(&0x100), "hot entry retained");
        assert!(!cache.entries.contains_key(&0x101), "oldest entry evicted");
        assert!(cache.entries.contains_key(&0x200), "new entry admitted");
    }
}
