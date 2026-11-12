// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![expect(clippy::expect_used, reason = "test setup")]

use rx82::system::System;

#[test]
fn command_line_defaults_to_the_available_backend() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rx82"))
        .arg("--help")
        .output()
        .expect("run RX82 help");
    assert!(output.status.success());
    let default = if cfg!(r8jit_native) {
        "jit"
    } else {
        "interpreter"
    };
    assert!(String::from_utf8_lossy(&output.stdout).contains(&format!("[default: {default}]")));
}

#[test]
fn invalid_selection_preserves_cpu_and_default_interpreter() {
    let mut system = System::default();
    system.cpu.pc = 0x1234;
    assert!(system.select_backend("unknown").is_err());
    assert_eq!(system.cpu.pc, 0x1234);
    assert_eq!(system.backend.native_entries(), 0);
    system
        .select_backend("interpreter")
        .expect("interpreter available");
    assert_eq!(system.cpu.pc, 0x1234);
}

#[cfg(not(r8jit_native))]
#[test]
fn explicit_jit_request_reports_unavailability() {
    let mut system = System::default();
    let error = system
        .select_backend("jit")
        .expect_err("unavailable backend");
    assert_eq!(
        error.to_string(),
        "the JIT backend is unavailable in this build"
    );
}

#[test]
fn command_line_rejects_unknown_backend_before_starting() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rx82"))
        .args(["--backend", "unknown", "--skiprom"])
        .output()
        .expect("run RX82");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown CPU backend: unknown"));
}
