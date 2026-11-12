// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::{bus::Bus, cpu::Cpu, system::Device as _};
use anyhow::{Result, bail};

/// CPU execution selection. The interpreter remains the default.
#[derive(Default)]
pub enum Backend {
    #[default]
    Interpreter,
    #[cfg(r8jit_native)]
    Jit(crate::jit::Jit),
}

#[expect(
    clippy::arbitrary_source_item_ordering,
    reason = "selection precedes execution helpers"
)]
#[expect(
    clippy::pattern_type_mismatch,
    reason = "borrow enum payloads with match ergonomics"
)]
impl Backend {
    /// Returns cached blocks and allocated executable bytes.
    #[must_use]
    pub fn cache_usage(&self) -> (usize, u64) {
        match self {
            Self::Interpreter => (0, 0),
            #[cfg(r8jit_native)]
            Self::Jit(jit) => jit.cache_usage(),
        }
    }

    /// Releases cached code without changing the current CPU or bus state.
    ///
    /// # Errors
    /// Returns an error if the operating system cannot release code mappings.
    pub fn clear_cache(&mut self) -> Result<()> {
        match self {
            Self::Interpreter => Ok(()),
            #[cfg(r8jit_native)]
            Self::Jit(jit) => jit.clear_cache(),
        }
    }
    /// Creates the requested backend without changing CPU state.
    ///
    /// # Errors
    /// Returns an error if the backend is unknown, unavailable or cannot be created.
    pub fn select(name: &str) -> Result<Self> {
        match name {
            "interpreter" => Ok(Self::Interpreter),
            #[cfg(r8jit_native)]
            "jit" => Ok(Self::Jit(crate::jit::Jit::new()?)),
            #[cfg(not(r8jit_native))]
            "jit" => bail!("the JIT backend is unavailable in this build"),
            _ => bail!("unknown CPU backend: {name}"),
        }
    }

    /// Returns completed native CPU cycles, excluding Rust fallback.
    #[must_use]
    pub fn native_entries(&self) -> u64 {
        match self {
            Self::Interpreter => 0,
            #[cfg(r8jit_native)]
            Self::Jit(jit) => jit.native_entries(),
        }
    }

    pub(crate) fn tick(&mut self, cpu: &mut Cpu, bus: &mut Bus) {
        match self {
            Self::Interpreter => cpu.tick(bus),
            #[cfg(r8jit_native)]
            Self::Jit(jit) => jit.tick(cpu, bus),
        }
    }
}
