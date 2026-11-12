#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Zayn Otley
# https://ZaynOtley.com
# https://IntuitionEngine.io
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check the native JIT build policy and extracted Cargo distribution."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class CargoJitTests(unittest.TestCase):
    def test_native_flags_reach_the_go_archive(self):
        with tempfile.TemporaryDirectory(prefix="r8-native-flags-") as directory:
            compiler = Path(directory) / "cc"
            compiler.write_text('#!/bin/sh\nprintf \'%s\\n\' "$@" >> "$R8JIT_CC_LOG"\nexec cc "$@"\n')
            compiler.chmod(0o755)
            log = Path(directory) / "compiler.log"
            environment = os.environ | {
                "CARGO_TARGET_DIR": directory, "GOAMD64": "v1",
                "CGO_CFLAGS": "-g", "R8JIT_PGO": "",
                "CC": str(compiler), "R8JIT_CC_LOG": str(log),
            }
            environment.pop("R8JIT_NATIVE", None)
            messages = subprocess.check_output(
                ["cargo", "build", "--offline", "-p", "rx82",
                 "--message-format=json"], cwd=ROOT, env=environment, text=True, timeout=300,
            )
            libraries = [Path(message["out_dir"]) / "libr8jit.a"
                         for line in messages.splitlines()
                         if (message := json.loads(line))["reason"] == "build-script-executed"
                         and (Path(message["out_dir"]) / "libr8jit.a").is_file()]
            self.assertEqual(len(libraries), 1)
            settings = subprocess.check_output(
                ["go", "version", "-m", str(Path(directory) / "debug/rx82")], text=True,
            )
            self.assertIn("-buildmode=c-archive", settings)
            arguments = log.read_text().splitlines()
            for flag in ("-Ofast", "-march=native", "-mtune=native", "-g"):
                self.assertIn(flag, arguments)
            self.assertIn("GOAMD64=v1", settings)
            log.write_text("")
            environment["R8JIT_NATIVE"] = "0"
            subprocess.run(
                ["cargo", "build", "--offline", "-p", "rx82", "--features", "jit"],
                cwd=ROOT, env=environment, check=True, timeout=300,
            )
            arguments = log.read_text().splitlines()
            for flag in ("-Ofast", "-march=native", "-mtune=native"):
                self.assertNotIn(flag, arguments)
            environment["R8JIT_PGO"] = str(Path(directory) / "missing.pprof")
            result = subprocess.run(
                ["cargo", "build", "--offline", "-p", "rx82", "--features", "jit"],
                cwd=ROOT, env=environment, capture_output=True, text=True, timeout=300,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("R8JIT_PGO must name an existing absolute profile path", result.stderr)

    def test_packaged_jit_builds_and_runs(self):
        with tempfile.TemporaryDirectory(prefix="r8-cargo-package-") as directory:
            directory = Path(directory)
            subprocess.run(
                ["cargo", "package", "--workspace", "--allow-dirty", "--no-verify",
                 "--offline", "--target-dir", str(directory / "artefacts")],
                cwd=ROOT, check=True, timeout=120,
            )
            packages = {}
            for name in ("r8cpu", "r8asm", "rx82"):
                archives = list((directory / "artefacts/package").glob(f"{name}-*.crate"))
                self.assertEqual(len(archives), 1, name)
                with tarfile.open(archives[0]) as archive:
                    archive.extractall(directory, filter="data")
                packages[name] = archives[0].stem
            compiler = directory / packages["rx82"] / "jit"
            crate = directory / packages["rx82"]
            for name in ("LICENSE", "LICENSING.md", "LICENSES/AGPL-3.0-or-later.txt"):
                packaged = crate / name
                self.assertTrue(packaged.is_file(), f"Cargo package omitted {name}")
                self.assertEqual(packaged.read_bytes(), (ROOT / name).read_bytes())
            for name in ("go.mod", "r8jit.h", "compiler.go", "native_probe_linux_amd64.S"):
                self.assertTrue((compiler / name).is_file(), f"Cargo package omitted jit/{name}")
            # Resolve sibling dependencies from their extracted packages, without publishing.
            (directory / "Cargo.toml").write_text(
                '[workspace]\nresolver = "2"\nmembers = ['
                + ", ".join(f'"{name}"' for name in packages.values())
                + ']\n[patch.crates-io]\n'
                + "".join(f'{name} = {{ path = "{packages[name]}" }}\n'
                          for name in ("r8cpu", "r8asm"))
            )
            subprocess.run(
                ["cargo", "build", "--offline", "-p", "rx82", "--features", "jit"],
                cwd=directory, check=True, timeout=300,
            )
            program = directory / "halt.bin"
            program.write_bytes(b"\x00")
            environment = os.environ.copy()
            for variable in ("LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT"):
                environment.pop(variable, None)
            subprocess.run(
                [str(directory / "target/debug/rx82"), "--skiprom", "--turbo",
                 "--backend", "jit", str(program)],
                cwd=directory, env=environment, check=True, timeout=30,
            )
            subprocess.run(
                ["cargo", "install", "--offline", "--debug", "--path",
                 packages["rx82"], "--root", str(directory / "installed")],
                cwd=directory, env=environment, check=True, timeout=300,
            )
            shutil.rmtree(directory / "target")
            subprocess.run(
                [str(directory / "installed/bin/rx82"), "--skiprom", "--turbo",
                 "--backend", "jit", str(program)],
                cwd=directory, env=environment, check=True, timeout=30,
            )

    def test_static_crt_is_rejected_before_go(self):
        host = next(line.split(": ", 1)[1] for line in subprocess.check_output(
            ["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
        with tempfile.TemporaryDirectory(prefix="r8-static-crt-") as directory:
            environment = os.environ | {
                "CARGO_TARGET_DIR": directory,
                "RUSTFLAGS": "-C target-feature=+crt-static",
                "GOROOT": "/nonexistent-jit-toolchain",
            }
            result = subprocess.run(
                ["cargo", "check", "--offline", "-p", "rx82", "--lib", "--features",
                 "jit", "--target", host],
                cwd=ROOT, env=environment, capture_output=True, text=True, timeout=300,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("the Linux amd64 JIT requires a dynamic CRT", result.stderr)
            self.assertNotIn("Go JIT archive build failed", result.stderr)


if __name__ == "__main__":
    unittest.main()
