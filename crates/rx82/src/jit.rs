// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

#![expect(
    clippy::self_named_module_files,
    reason = "match existing source file layout"
)]
#![expect(
    clippy::arbitrary_source_item_ordering,
    reason = "keep compiler lifecycle and test fixtures together"
)]
#![cfg_attr(
    not(test),
    expect(
        clippy::single_call_fn,
        reason = "separate conversion and lifecycle responsibilities"
    )
)]
#![expect(
    clippy::pattern_type_mismatch,
    reason = "use idiomatic match ergonomics"
)]

use crate::{bus::Bus, cpu::Cpu, state::State, system::Device as _};
mod context;
mod region;
use anyhow::{Result, bail};
use r8cpu::instructions::InstructionKind;
use std::collections::HashMap;
extern crate alloc;
use alloc::collections::VecDeque;

/// R8 instructions contain at most three bytes. Unused bytes are always zero.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
struct Fetched {
    bytes: [u8; 3],
    length: u8,
}

impl Fetched {
    fn clear(&mut self) {
        *self = Self::default();
    }
    fn push(&mut self, value: u8) {
        if let Some(byte) = self.bytes.get_mut(usize::from(self.length)) {
            *byte = value;
            self.length = self.length.wrapping_add(1);
        } else {
            self.clear();
        }
    }
    fn as_slice(&self) -> &[u8] {
        self.bytes
            .get(..usize::from(self.length))
            .unwrap_or_default()
    }
}

#[allow(
    warnings,
    clippy::all,
    clippy::pedantic,
    clippy::restriction,
    reason = "generated from the authoritative C header"
)]
pub(crate) mod ffi {
    include!(concat!(env!("OUT_DIR"), "/r8jit.rs"));
}

/// Synchronous native compiler and fetched-instruction cache.
pub struct Jit {
    handle: u64,
    regions: Option<region::Regions>,
    cache: HashMap<Fetched, usize>,
    order: VecDeque<Fetched>,
    fetched: Fetched,
    expected: Option<State>,
    entries: u64,
    active: bool,
    allocated: u64,
}

impl Jit {
    pub(crate) fn new() -> Result<Self> {
        let mut handle = 0;
        let mut diagnostic = [0_i8; 256];
        // SAFETY: outputs are valid for the call. Go retains no Rust pointers.
        let status = unsafe {
            ffi::r8jit_create(
                ffi::R8JIT_ABI_VERSION,
                64 * 4096,
                &raw mut handle,
                diagnostic.as_mut_ptr(),
                256,
            )
        };
        if status != 0_i32 {
            bail!("JIT compiler creation failed ({status})");
        }
        Ok(Self {
            handle,
            regions: None,
            cache: HashMap::new(),
            order: VecDeque::new(),
            fetched: Fetched::default(),
            expected: None,
            entries: 0,
            active: false,
            allocated: 0,
        })
    }

    #[must_use]
    pub fn native_entries(&self) -> u64 {
        self.entries
    }

    pub(crate) fn cache_usage(&self) -> (usize, u64) {
        let (regions, bytes) = self
            .regions
            .as_ref()
            .map_or((0, 0), region::Regions::cache_usage);
        (
            self.cache.len().saturating_add(regions),
            self.allocated.saturating_add(bytes),
        )
    }

    pub(crate) fn clear_cache(&mut self) -> Result<()> {
        self.regions = None;
        self.fetched.clear();
        self.expected = None;
        self.active = false;
        self.reclaim()
    }

    fn reclaim(&mut self) -> Result<()> {
        self.cache.clear();
        self.order.clear();
        self.active = false;
        // SAFETY: all entry references were cleared and execution is stopped.
        let status = unsafe { ffi::r8jit_invalidate(self.handle, core::ptr::null_mut(), 0) };
        if status != 0_i32 {
            bail!("JIT cache reclamation failed ({status})");
        }
        self.allocated = 0;
        Ok(())
    }

    pub(crate) fn run_region(
        &mut self,
        cpu: &mut Cpu,
        bus: &mut Bus,
        code: &[u8],
        budget: u32,
        ram: Option<&mut crate::memory::Memory>,
    ) -> u32 {
        if self.regions.is_none() {
            self.regions = region::Regions::new().ok();
        }
        let cycles = self
            .regions
            .as_mut()
            .map_or(0, |regions| regions.run(cpu, bus, code, budget, ram));
        if cycles != 0 {
            self.entries = self.entries.wrapping_add(u64::from(cycles));
            self.expected = None;
            self.active = false;
            self.fetched.clear();
        }
        cycles
    }

    fn evict(&mut self) -> Result<()> {
        if let Some(key) = self.order.front().copied()
            && let Some(entry) = self.cache.get(&key).copied()
        {
            // SAFETY: execution is stopped; no entry runs during eviction.
            let status = unsafe { ffi::r8jit_release(self.handle, entry) };
            if status != 0_i32 {
                bail!("JIT entry release failed ({status})");
            }
            // Every single-instruction entry occupies one native page.
            let bytes = self
                .allocated
                .checked_div(u64::try_from(self.cache.len()).unwrap_or(1))
                .unwrap_or(0);
            self.cache.remove(&key);
            self.order.pop_front();
            self.allocated = self.allocated.saturating_sub(bytes);
        }
        Ok(())
    }

    pub(crate) fn tick(&mut self, cpu: &mut Cpu, bus: &mut Bus) {
        let before = cpu.state;
        if self.expected != Some(before) {
            self.fetched.clear();
            self.active = false;
        }
        match before {
            State::FetchOpcode => self.fetched.clear(),
            State::Decode if self.expected == Some(before) => {
                self.fetched.clear();
                self.fetched.push(bus.data);
            }
            State::ReadOp | State::ReadOpLo | State::ReadOpHi if self.fetched.length != 0 => {
                self.fetched.push(bus.data);
            }
            _ => (),
        }
        let instruction_matches = before == State::Execute
            && self.fetched.as_slice().first().is_some_and(|opcode| {
                InstructionKind::try_from(*opcode).is_ok_and(|instruction| instruction == cpu.ins)
            });
        let operands_match = match self.fetched.as_slice() {
            [_] => true,
            [_, lo] => *lo == cpu.op_lo && cpu.op_hi == 0,
            [_, lo, hi] => *lo == cpu.op_lo && *hi == cpu.op_hi,
            _ => false,
        };
        let native = (instruction_matches && operands_match) || self.active;
        if native && self.try_native(cpu, bus) {
            self.active = cpu.state != State::FetchOpcode;
            #[cfg(feature = "states")]
            println!("    {} -> {}", before, cpu.state);
        } else {
            self.active = false;
            cpu.tick(bus);
        }
        self.expected = Some(cpu.state);
    }

    #[expect(
        clippy::semicolon_outside_block,
        reason = "keep the unsafe call as a statement"
    )]
    fn try_native(&mut self, cpu: &mut Cpu, bus: &mut Bus) -> bool {
        let entry = if let Some(entry) = self.cache.get(&self.fetched) {
            *entry
        } else {
            if self.cache.len() >= 64 && self.evict().is_err() {
                return false;
            }
            let mut entry = 0;
            let mut allocated = 0;
            // SAFETY: input and outputs live through the synchronous compilation call.
            let status = unsafe {
                ffi::r8jit_compile(
                    self.handle,
                    self.fetched.bytes.as_mut_ptr(),
                    u32::from(self.fetched.length),
                    &raw mut entry,
                    &raw mut allocated,
                    core::ptr::null_mut(),
                    0,
                )
            };
            if status != 0_i32 {
                return false;
            }
            self.cache.insert(self.fetched, entry);
            self.order.push_back(self.fetched);
            self.allocated = self.allocated.wrapping_add(allocated);
            entry
        };
        let mut context = context::encode(cpu, bus);
        // SAFETY: entry is RX code owned by this live compiler. Its System V
        // signature is defined in r8jit.h. Invalidation cannot overlap this call.
        let run: unsafe extern "C" fn(*mut ffi::r8_context) =
            unsafe { core::mem::transmute(entry) };
        // SAFETY: this Rust-owned C-layout context stays on the stack and is
        // accessed exclusively by native code until the synchronous call returns.
        unsafe {
            run(&raw mut context);
        }
        if !context::apply(&context, cpu, bus) {
            return false;
        }
        self.entries = self.entries.wrapping_add(1);
        true
    }
}

impl Drop for Jit {
    fn drop(&mut self) {
        self.cache.clear();
        self.order.clear();
        // SAFETY: dropping requires exclusive ownership; no native call is active.
        unsafe {
            ffi::r8jit_destroy(self.handle, core::ptr::null_mut(), 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::Backend, system::System};
    use core::{fmt::Write as _, hint::black_box};
    use std::time::Instant;

    #[test]
    #[expect(clippy::expect_used, reason = "failure-path fixture")]
    fn compiler_budget_failure_falls_back_for_exactly_one_cycle() {
        let mut jit = Jit::new().expect("compiler instance");
        assert_eq!(
            // SAFETY: the fresh instance has no published entries or active calls.
            unsafe { ffi::r8jit_destroy(jit.handle, core::ptr::null_mut(), 0) },
            0_i32,
            "replace unused compiler"
        );
        assert_eq!(
            // SAFETY: the output lives through the call; one byte cannot hold a code page.
            unsafe {
                ffi::r8jit_create(
                    ffi::R8JIT_ABI_VERSION,
                    1,
                    &raw mut jit.handle,
                    core::ptr::null_mut(),
                    0,
                )
            },
            0_i32,
            "small-budget compiler"
        );
        let mut reference = System {
            turbo: true,
            ..System::default()
        };
        let mut native = System {
            turbo: true,
            backend: Backend::Jit(jit),
            ..System::default()
        };
        for system in [&mut reference, &mut native] {
            system.cpu.pc = 0x0100;
            system.mem.load(0x0100, &[0x10, 2]).expect("program fits");
        }
        for _ in 0_u8..6 {
            reference.tick();
            native.tick();
            assert_eq!(native.cpu.state, reference.cpu.state, "fallback state");
            assert_eq!(
                format!("{:?}", native.cpu),
                format!("{:?}", reference.cpu),
                "fallback CPU"
            );
            assert_eq!(
                format!("{:?}", native.bus),
                format!("{:?}", reference.bus),
                "fallback bus"
            );
        }
        assert_eq!(native.cycles, 6, "no extra scheduler ticks");
        assert_eq!(
            native.cpu.regs.get(r8cpu::regs::Reg::A),
            2,
            "current instruction executed"
        );
        assert_eq!(
            native.backend.native_entries(),
            0,
            "failed compilation did not run native code"
        );
        assert_eq!(
            native.backend.cache_usage(),
            (0, 0),
            "failed block did not publish a mapping"
        );
    }

    #[test]
    #[ignore = "manual release-mode feasibility measurement"]
    #[expect(
        clippy::expect_used,
        clippy::panic,
        clippy::shadow_unrelated,
        reason = "bounded benchmark setup and timing scopes"
    )]
    fn early_slice_feasibility() {
        const CYCLES: u32 = 1_000_000;
        let setup = || {
            let mut system = System {
                turbo: true,
                ..System::default()
            };
            system.cpu.pc = 0x0100;
            system
                .mem
                .load(0x0100, &[0x50, 1, 0xF0, 0xFC])
                .expect("loop fits");
            system
        };
        let mut reference = setup();
        let mut native = setup();
        let start = Instant::now();
        native.select_backend("jit").expect("JIT available");
        let creation = start.elapsed();
        let start = Instant::now();
        for _ in 0_u32..12 {
            native.tick();
        }
        let cold = start.elapsed();
        native.cpu = Cpu::default();
        native.cpu.pc = 0x0100;
        native.cycles = 0;
        let start = Instant::now();
        for _ in 0..CYCLES {
            reference.tick();
        }
        let interpreter = start.elapsed();
        let before_entries = native.backend.native_entries();
        let start = Instant::now();
        for _ in 0..CYCLES {
            native.tick();
        }
        let steady = start.elapsed();
        assert_eq!(native.cpu.state, reference.cpu.state);
        assert_eq!(format!("{:?}", native.cpu), format!("{:?}", reference.cpu));
        assert_eq!(format!("{:?}", native.bus), format!("{:?}", reference.bus));
        assert_eq!(native.mem.data, reference.mem.data);
        assert_eq!(native.cycles, reference.cycles);
        let start = Instant::now();
        let mut context = context::encode(&native.cpu, &native.bus);
        for _ in 0..CYCLES {
            context = context::encode(black_box(&native.cpu), black_box(&native.bus));
            assert!(context::apply(
                black_box(&context),
                &mut native.cpu,
                &mut native.bus
            ));
        }
        let conversion = start.elapsed();
        let Backend::Jit(jit) = &mut native.backend else {
            panic!("JIT selected");
        };
        native.cpu.state = State::Execute;
        native.cpu.ins = InstructionKind::Nop;
        jit.fetched.clear();
        jit.fetched.push(1);
        assert!(jit.try_native(&mut native.cpu, &mut native.bus));
        let entry = *jit.cache.get(&jit.fetched).expect("compiled NOP");
        // SAFETY: the live compiler owns this NOP entry and context has C layout.
        let run: unsafe extern "C" fn(*mut ffi::r8_context) =
            unsafe { core::mem::transmute(entry) };
        let start = Instant::now();
        for _ in 0..CYCLES {
            context.state = u16::try_from(ffi::R8_EXECUTE).expect("ABI state fits");
            context.ins = 1;
            // SAFETY: no invalidation or concurrent execution occurs in this loop.
            unsafe {
                run(black_box(&raw mut context));
            }
        }
        let entry_time = start.elapsed();
        let mut compiler = Jit::new().expect("compilation benchmark instance");
        let start = Instant::now();
        for operand in 0_u8..32 {
            let mut code = [0x10, operand];
            let mut address = 0;
            let mut bytes = 0;
            // SAFETY: all buffers live through the synchronous compilation call.
            let status = unsafe {
                ffi::r8jit_compile(
                    compiler.handle,
                    code.as_mut_ptr(),
                    2,
                    &raw mut address,
                    &raw mut bytes,
                    core::ptr::null_mut(),
                    0,
                )
            };
            assert_eq!(status, 0_i32, "compile benchmark block");
            assert_ne!(address, 0, "published entry");
            compiler.allocated = compiler.allocated.wrapping_add(bytes);
        }
        let compilation = start.elapsed();
        println!(
            "cycles={CYCLES} creation_ns={} cold_12_cycles_ns={} interpreter_ns={} jit_ns={} conversion_ns={} native_entry_ns={} native_cycles={} cache_entries={}",
            creation.as_nanos(),
            cold.as_nanos(),
            interpreter.as_nanos(),
            steady.as_nanos(),
            conversion.as_nanos(),
            entry_time.as_nanos(),
            jit.entries.wrapping_sub(before_entries).wrapping_sub(1),
            jit.cache.len()
        );
        println!(
            "compilation_32_blocks_ns={} compilation_bytes={} steady_cache_bytes={}",
            compilation.as_nanos(),
            compiler.allocated,
            jit.allocated
        );
    }

    #[test]
    #[ignore = "manual release-mode workload measurement after compatibility acceptance"]
    #[expect(
        clippy::expect_used,
        clippy::panic,
        reason = "bounded workload measurement"
    )]
    fn supplied_workload_performance() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for name in ["countdown", "memcpy"] {
            let program =
                r8asm::assemble_source_file(root.join(format!("../r8asm/examples/{name}.asm")))
                    .expect("assemble supplied workload");
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
                system.mem.load(0x0100, &program).expect("workload fits");
                for value in 0_u8..=254 {
                    system
                        .mem
                        .set(0x1000_u16.wrapping_add(u16::from(value)), value);
                }
            }
            let initial_memory = reference.mem.data.clone();
            let run = |system: &mut System| {
                let start = Instant::now();
                for cycles in 1_u32..=15_000_000 {
                    system.tick();
                    if system.cpu.halt {
                        return (start.elapsed(), cycles);
                    }
                }
                panic!("workload did not halt");
            };
            let (interpreter, reference_cycles) = run(&mut reference);
            let (cold, cold_cycles) = run(&mut native);
            assert_eq!(cold_cycles, reference_cycles, "cold cycles");
            assert_eq!(
                format!("{:?}", native.cpu),
                format!("{:?}", reference.cpu),
                "cold CPU"
            );
            assert_eq!(native.mem.data, reference.mem.data, "cold RAM");
            native.cpu = Cpu::default();
            native.cpu.pc = 0x0100;
            native.bus = Bus::default();
            native.cycles = 0;
            native.mem.data.clone_from(&initial_memory);
            let before_entries = native.backend.native_entries();
            let (warm, warm_cycles) = run(&mut native);
            assert_eq!(warm_cycles, reference_cycles, "warm cycles");
            assert_eq!(native.cpu.state, reference.cpu.state, "warm state");
            assert_eq!(
                format!("{:?}", native.cpu),
                format!("{:?}", reference.cpu),
                "warm CPU"
            );
            assert_eq!(
                format!("{:?}", native.bus),
                format!("{:?}", reference.bus),
                "warm bus"
            );
            assert_eq!(native.mem.data, reference.mem.data, "warm RAM");
            let (entries, bytes) = native.backend.cache_usage();
            let mut code = String::new();
            for byte in &program {
                write!(&mut code, "{byte:02x}").expect("workload hex");
            }
            println!(
                "workload={name} code_hex={code} cycles={reference_cycles} interpreter_ns={} cold_jit_ns={} warm_jit_ns={} native_cycles={} cache_entries={entries} cache_bytes={bytes}",
                interpreter.as_nanos(),
                cold.as_nanos(),
                warm.as_nanos(),
                native.backend.native_entries().wrapping_sub(before_entries)
            );
        }
    }
}
