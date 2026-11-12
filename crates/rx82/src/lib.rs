// SPDX-FileCopyrightText: 2026 John Arundel
// SPDX-FileCopyrightText: 2026 Zayn Otley (modifications)
// https://ZaynOtley.com
// https://IntuitionEngine.io

#![cfg_attr(doc, doc = include_str!("../README.md"))]

pub mod backend;
pub mod bus;
pub mod clock;
pub mod cpu;
#[cfg(r8jit_native)]
pub mod jit;
pub mod memory;
pub mod monitor;
pub mod rom;
pub mod state;
pub mod system;
