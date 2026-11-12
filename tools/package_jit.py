#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Zayn Otley
# https://ZaynOtley.com
# https://IntuitionEngine.io
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build and verify a relocatable Linux amd64 RX82 JIT package."""

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def run(arguments, **kwargs):
    return subprocess.run(arguments, check=True, capture_output=True, text=True, **kwargs).stdout


def portable_environment():
    environment = os.environ | {
        "R8JIT_NATIVE": "0", "GOAMD64": "v1",
        "CGO_CFLAGS": "-O2 -march=x86-64 -mtune=generic", "CGO_LDFLAGS": "",
        "GOFLAGS": "",
        "RUSTFLAGS": "-C target-cpu=x86-64 -C target-feature=-crt-static",
    }
    for variable in ("CARGO_ENCODED_RUSTFLAGS", "R8JIT_PGO"):
        environment.pop(variable, None)
    return environment


def verify_package(output):
    environment = os.environ.copy()
    for variable in ("LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT"):
        environment.pop(variable, None)
    with tempfile.TemporaryDirectory(prefix="r8-jit-run-") as directory:
        directory = Path(directory)
        # A copied executable must run without any build or package directory.
        executable = directory / "rx82"
        shutil.copy2(output / "rx82", executable)
        program = directory / "halt.bin"
        program.write_bytes(b"\x00")
        run([str(executable), "--skiprom", "--turbo", "--backend", "jit", str(program)],
            cwd=directory, env=environment, timeout=30)
        run([str(executable), "--help"], cwd=directory, env=environment, timeout=30)


def snapshot(source, notices):
    for name in ("Cargo.toml", "Cargo.lock", "LICENSE", "LICENSING.md", "R8_X64_JIT_PERFORMANCE.json"):
        shutil.copy2(ROOT / name, source / name)
    for name in ("crates", "tools", "LICENSES"):
        shutil.copytree(ROOT / name, source / name, ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "target", "dist"))
    configuration = run(["cargo", "vendor", "--locked", "--versioned-dirs", str(source / "vendor")], cwd=ROOT)
    configuration = configuration.replace(str(source / "vendor"), "vendor")
    (source / ".cargo").mkdir()
    (source / ".cargo/config.toml").write_text(configuration)
    dependencies = []
    for crate in sorted((source / "vendor").iterdir()):
        metadata = tomllib.loads((crate / "Cargo.toml").read_text())["package"]
        dependencies.append({"name": metadata["name"], "version": metadata["version"],
                             "licence": metadata.get("license", metadata.get("license-file", "see source"))})
        for path in crate.rglob("*"):
            if path.is_file() and any(word in path.name.lower() for word in ("license", "licence", "copying", "copyright", "notice", "authors")):
                destination = notices / "dependencies" / crate.name / path.relative_to(crate)
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, destination)
    return dependencies


def toolchain_notices(notices, go_license=None, rust_notices=None):
    go_root = Path(run(["go", "env", "GOROOT"]).strip())
    shutil.copy2(go_license or go_root / "LICENSE", notices / "Go-LICENSE.txt")
    rust_root = Path(run(["rustc", "--print", "sysroot"]).strip())
    version = ".".join(run(["rustc", "--version"]).split()[1].split(".")[:2])
    candidates = [rust_root / "share/doc/rust", Path(f"/usr/share/licenses/rust{version}")]
    if rust_notices is not None:
        candidates.insert(0, rust_notices)
    for candidate in candidates:
        if ((candidate / "COPYRIGHT.html").is_file()
                and all((candidate / "licenses" / name).is_file()
                        for name in ("MIT.txt", "Apache-2.0.txt"))):
            destination = notices / "Rust"
            destination.mkdir()
            for path in candidate.glob("COPYRIGHT*.html"):
                shutil.copy2(path, destination / path.name)
            shutil.copytree(candidate / "licenses", destination / "licenses")
            return
        if all((candidate / name).is_file() for name in ("COPYRIGHT", "LICENSE-MIT", "LICENSE-APACHE")):
            for name in ("COPYRIGHT", "LICENSE-MIT", "LICENSE-APACHE"):
                shutil.copy2(candidate / name, notices / f"Rust-{name}.txt")
            return
    raise RuntimeError("Rust copyright notices and MIT/Apache licence texts are required for packaging")


def archive(source, destination):
    def normalise(info):
        info.uid = info.gid = info.mtime = 0
        info.uname = info.gname = ""
        return info
    with destination.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w") as bundle:
                for path in sorted(source.rglob("*")):
                    bundle.add(path, arcname=Path("source") / path.relative_to(source),
                               recursive=False, filter=normalise)


def verify_source(source, target):
    with tempfile.TemporaryDirectory(prefix="r8-jit-rebuild-") as directory:
        directory = Path(directory)
        environment = portable_environment() | {
            "CARGO_HOME": str(directory / "cargo-home"),
            "CARGO_TARGET_DIR": str(directory / "build"),
            "CARGO_NET_OFFLINE": "true", "GOPROXY": "off", "GOTOOLCHAIN": "local",
        }
        run(["cargo", "build", "--locked", "--release", "--offline", "-p", "rx82",
             "--features", "jit", "--target", target], cwd=source, env=environment, timeout=300)
        executable = directory / "build" / target / "release/rx82"
        program = directory / "halt.bin"
        program.write_bytes(b"\x00")
        run([str(executable), "--skiprom", "--turbo", "--backend", "jit", str(program)],
            cwd=directory, env=environment, timeout=30)


def package(output, go_license=None, rust_notices=None):
    if platform.system() != "Linux" or platform.machine().lower() not in ("x86_64", "amd64"):
        raise RuntimeError("packaging requires native Linux amd64")
    if output.is_relative_to(ROOT) and not output.is_relative_to(ROOT / "dist"):
        raise RuntimeError("use dist/ or a directory outside the repository")
    output.mkdir(parents=True, exist_ok=False)
    target = next(line.split(": ", 1)[1] for line in run(["rustc", "-vV"]).splitlines() if line.startswith("host: "))
    environment = portable_environment()
    messages = run(["cargo", "build", "--locked", "--release", "-p", "rx82", "--features", "jit",
                    "--target", target, "--message-format=json"], cwd=ROOT, env=environment)
    executable = None
    for line in messages.splitlines():
        message = json.loads(line)
        if message["reason"] == "compiler-artifact" and message["target"]["name"] == "rx82" and message.get("executable"):
            executable = Path(message["executable"])
    if executable is None:
        raise RuntimeError("Cargo did not report the RX82 executable")
    shutil.copy2(executable, output / "rx82")
    verify_package(output)
    notices = output / "LICENSES"
    shutil.copytree(ROOT / "LICENSES", notices)
    shutil.copy2(ROOT / "LICENSE", notices / "R8-original-MIT.txt")
    shutil.copy2(ROOT / "LICENSING.md", output / "LICENSING.md")
    toolchain_notices(notices, go_license, rust_notices)
    with tempfile.TemporaryDirectory(prefix="r8-jit-source-") as directory:
        source = Path(directory)
        dependencies = snapshot(source, notices)
        verify_source(source, target)
        archive(source, output / "source.tar.gz")
    manifest = tomllib.loads((ROOT / "crates/rx82/Cargo.toml").read_text())["package"]
    metadata = {
        "name": manifest["name"], "version": manifest["version"], "target": target,
        "licence": manifest["license"],
        "revision": run(["git", "rev-parse", "HEAD"], cwd=ROOT).strip(),
        "rust": run(["rustc", "--version"]).strip(), "go": run(["go", "version"]).strip(),
        "dependencies": dependencies,
        "cpu_settings": {name: environment[name] for name in
                         ("R8JIT_NATIVE", "GOAMD64", "CGO_CFLAGS", "RUSTFLAGS")},
        "offline_source_rebuild": True,
        "sha256": {name: hashlib.sha256((output / name).read_bytes()).hexdigest()
                   for name in ("rx82", "source.tar.gz")},
    }
    (output / "package.json").write_text(json.dumps(metadata, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "dist/rx82-linux-amd64")
    parser.add_argument("--go-license", type=Path, help="Go LICENSE file when GOROOT does not contain it")
    parser.add_argument("--rust-notices", type=Path, help="Rust toolchain COPYRIGHT and licence directory")
    arguments = parser.parse_args()
    try:
        package(arguments.output.resolve(), arguments.go_license, arguments.rust_notices)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        detail = error.stderr if isinstance(error, subprocess.CalledProcessError) else str(error)
        parser.exit(1, f"RX82 packaging failed: {detail}\n")


if __name__ == "__main__":
    main()
