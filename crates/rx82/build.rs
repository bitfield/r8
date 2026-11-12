// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
mod build_support;

#[cfg(feature = "jit")]
#[expect(
    clippy::expect_used,
    reason = "abort the build with a concrete toolchain diagnostic"
)]
#[expect(
    clippy::single_call_fn,
    reason = "separate feature-specific build work"
)]
fn build_jit() {
    use std::{env, path::PathBuf, process::Command};

    let source =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies manifest directory"))
            .join("jit");
    let output = PathBuf::from(env::var("OUT_DIR").expect("Cargo supplies OUT_DIR"));
    println!("cargo:rerun-if-changed={}", source.display());
    for name in [
        "CC",
        "CGO_CFLAGS",
        "CGO_LDFLAGS",
        "GOCACHE",
        "GOFLAGS",
        "GOAMD64",
        "R8JIT_NATIVE",
        "R8JIT_PGO",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let mut command = Command::new("go");
    command
        .current_dir(&source)
        .env("GOOS", "linux")
        .env("GOARCH", "amd64")
        .env("CGO_ENABLED", "1")
        .args([
            "build",
            "-trimpath",
            "-buildvcs=false",
            "-buildmode=c-archive",
            "-o",
        ])
        .arg(output.join("libr8jit.a"));
    if env::var("R8JIT_NATIVE").as_deref() != Ok("0") {
        let existing = env::var("CGO_CFLAGS").unwrap_or_default();
        command.env(
            "CGO_CFLAGS",
            format!("-Ofast -march=native -mtune=native {existing}"),
        );
    }
    if let Some(profile) = env::var_os("R8JIT_PGO").filter(|value| !value.is_empty()) {
        let profile = PathBuf::from(profile);
        assert!(
            profile.is_absolute() && profile.is_file(),
            "R8JIT_PGO must name an existing absolute profile path"
        );
        println!("cargo:rerun-if-changed={}", profile.display());
        command.arg("-pgo").arg(profile);
    } else {
        command.arg("-pgo=auto");
    }
    let status = command
        .arg(".")
        .status()
        .expect("JIT builds require Go 1.27 and a native C compiler");
    assert!(status.success(), "Go JIT archive build failed");
    bindgen::Builder::default()
        .header(source.join("r8jit.h").to_string_lossy())
        .allowlist_type("r8_context")
        .allowlist_type("r8_region")
        .allowlist_function("r8jit_.*")
        .allowlist_var("R8.*")
        .derive_default(true)
        .generate()
        .expect("JIT bindings require libclang")
        .write_to_file(output.join("r8jit.rs"))
        .expect("write JIT bindings");
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=static=r8jit");
    for library in ["pthread", "dl", "m"] {
        println!("cargo:rustc-link-lib={library}");
    }
    println!("cargo:rustc-cfg=r8jit_native");
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_support.rs");
    println!("cargo:rustc-check-cfg=cfg(r8jit_native)");
    if !build_support::native_target(
        cfg!(feature = "jit"),
        &std::env::var("HOST")?,
        &std::env::var("TARGET")?,
        &std::env::var("CARGO_CFG_TARGET_FEATURE")?,
    )? {
        return Ok(());
    }
    #[cfg(feature = "jit")]
    build_jit();
    Ok(())
}
