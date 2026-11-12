#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Zayn Otley
# https://ZaynOtley.com
# https://IntuitionEngine.io
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check portable build settings and installed toolchain notices."""

import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_jit


class PortablePackageTests(unittest.TestCase):
    def test_toolchain_notices_preserve_installed_layouts(self):
        layouts = (
            {"COPYRIGHT": "copyright", "LICENSE-MIT": "MIT", "LICENSE-APACHE": "Apache"},
            {"COPYRIGHT.html": "copyright", "COPYRIGHT-library.html": "library copyright",
             "licenses/MIT.txt": "MIT",
             "licenses/Apache-2.0.txt": "Apache", "licenses/LLVM-exception.txt": "LLVM"},
        )
        for layout in layouts:
            with self.subTest(layout=layout), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                rust = root / "share/doc/rust"
                for name, contents in layout.items():
                    path = rust / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(contents)
                go = root / "go"
                go.mkdir()
                (go / "LICENSE").write_text("Go")
                notices = root / "notices"
                notices.mkdir()
                with patch.object(package_jit, "run", side_effect=[str(go), str(root), "rustc 1.91.0"]):
                    package_jit.toolchain_notices(notices)
                self.assertEqual((notices / "Go-LICENSE.txt").read_text(), "Go")
                if "COPYRIGHT.html" in layout:
                    for name, contents in layout.items():
                        self.assertEqual((notices / "Rust" / name).read_text(), contents)
                else:
                    for name, contents in layout.items():
                        self.assertEqual((notices / f"Rust-{name}.txt").read_text(), contents)

    def test_portable_environment_overrides_host_tuning(self):
        inherited = {
            "R8JIT_NATIVE": "1", "GOAMD64": "v4",
            "CGO_CFLAGS": "-Ofast -march=native", "CGO_LDFLAGS": "-march=native",
            "GOFLAGS": "-ldflags=-linkmode=external",
            "RUSTFLAGS": "-C target-cpu=native",
            "CARGO_ENCODED_RUSTFLAGS": "-C\x1ftarget-cpu=native",
            "R8JIT_PGO": "/host/profile.pprof", "PATH": "/toolchain/bin",
        }
        with patch.dict(os.environ, inherited, clear=True):
            environment = package_jit.portable_environment()
        self.assertEqual(environment["R8JIT_NATIVE"], "0")
        self.assertEqual(environment["GOAMD64"], "v1")
        self.assertEqual(environment["CGO_CFLAGS"], "-O2 -march=x86-64 -mtune=generic")
        self.assertEqual(environment["CGO_LDFLAGS"], "")
        self.assertEqual(environment["GOFLAGS"], "")
        self.assertEqual(environment["RUSTFLAGS"],
                         "-C target-cpu=x86-64 -C target-feature=-crt-static")
        self.assertNotIn("CARGO_ENCODED_RUSTFLAGS", environment)
        self.assertNotIn("R8JIT_PGO", environment)
        self.assertEqual(environment["PATH"], inherited["PATH"])


if __name__ == "__main__":
    unittest.main()
