// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#![cfg(r8jit_native)]
#![expect(clippy::expect_used, reason = "subprocess test setup")]

#[test]
fn continuous_monitor_commands_use_native_regions() {
    use rx82::{monitor::Monitor, system::System};
    let program = [0x10, 3, 0x40, 0xF2, 0xFD, 0];
    for go_command in [false, true] {
        let mut reference = Monitor::default();
        reference.sys.turbo = true;
        reference.run_program(&program).expect("reference program");
        if go_command {
            reference.sys.debug_print();
        }
        let mut native = Monitor::default();
        native.sys = System {
            turbo: true,
            ..System::default()
        };
        native.sys.select_backend("jit").expect("JIT");
        if go_command {
            native.sys.mem.load(0x100, &program).expect("program fits");
            native.go(Some(0x100));
        } else {
            native.run_program(&program).expect("native program");
        }
        assert_eq!(
            format!("{:?}", native.sys.cpu),
            format!("{:?}", reference.sys.cpu)
        );
        assert_eq!(
            format!("{:?}", native.sys.bus),
            format!("{:?}", reference.sys.bus)
        );
        assert_eq!(native.sys.mem.data, reference.sys.mem.data);
        assert_eq!(native.sys.cycles, reference.sys.cycles);
        assert_eq!(native.last_addr, reference.last_addr);
        if !cfg!(feature = "states") {
            assert_eq!(
                native.sys.backend.native_entries(),
                40,
                "continuous execution includes native fetch and decode"
            );
        }
    }
}

#[test]
fn monitor_single_step_stops_after_one_instruction() {
    use r8cpu::regs::Reg::A;
    use rx82::monitor::Monitor;
    for backend in ["interpreter", "jit"] {
        let mut monitor = Monitor::default();
        monitor.sys.turbo = true;
        monitor.sys.select_backend(backend).expect("backend");
        monitor
            .sys
            .mem
            .load(0x100, &[0x10, 3, 0x40, 0])
            .expect("program fits");
        monitor.step(Some(0x100));
        assert_eq!(monitor.sys.cpu.pc, 0x102);
        assert_eq!(monitor.sys.cpu.regs.get(A), 3);
        // Register display reads three bytes through the bus after each step.
        assert_eq!(monitor.sys.cycles, 9);
        assert!(!monitor.sys.cpu.halt);
        monitor.step(None);
        assert_eq!(monitor.sys.cpu.pc, 0x103);
        assert_eq!(monitor.sys.cpu.regs.get(A), 2);
        assert_eq!(monitor.sys.cycles, 16);
        assert!(!monitor.sys.cpu.halt);
    }
}

#[test]
fn console_trap_outputs_the_same_character_exactly_once() {
    let path = std::env::temp_dir().join(format!("r8-jit-console-{}.bin", std::process::id()));
    for character in [b'Z', 0x80, 0xFF] {
        std::fs::write(&path, [0x1C, 0, 3, 0x10, character, 0xF9, 0x20, 0]).expect("write program");
        let run = |backend| {
            std::process::Command::new(env!("CARGO_BIN_EXE_rx82"))
                .args(["--skiprom", "--turbo", "--backend", backend])
                .arg(&path)
                .output()
                .expect("run RX82")
        };
        let reference = run("interpreter");
        let native = run("jit");
        assert!(reference.status.success(), "reference exit");
        assert!(native.status.success(), "native exit");
        assert_eq!(native.stdout, reference.stdout, "console bytes");
        let expected = char::from(character).to_string();
        assert_eq!(
            String::from_utf8(native.stdout)
                .expect("UTF-8 console")
                .matches(&expected)
                .count(),
            1,
            "one character event"
        );
    }
    std::fs::remove_file(path).expect("remove temporary program");
}

#[test]
fn monitor_step_register_and_memory_commands_match() {
    use std::io::Write as _;
    let run = |backend| {
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_rx82"))
            .args(["--skiprom", "--turbo", "--backend", backend])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("start monitor");
        child
            .stdin
            .take()
            .expect("monitor stdin")
            .write_all(b"R\nS 0100\nM 0100\nS\nQ\n")
            .expect("monitor commands");
        child.wait_with_output().expect("monitor exits")
    };
    let reference = run("interpreter");
    let native = run("jit");
    assert!(reference.status.success(), "reference monitor");
    assert!(native.status.success(), "native monitor");
    assert_eq!(native.stdout, reference.stdout, "monitor output");
    assert_eq!(native.stderr, reference.stderr, "monitor diagnostics");
}
