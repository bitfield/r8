// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg(test)]
#[path = "../build_support.rs"]
mod build_support;

#[test]
fn unsupported_targets_and_disabled_feature_need_no_jit_tools() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnux32",
    ] {
        assert_eq!(
            build_support::native_target(true, "x86_64-unknown-linux-gnu", target, "crt-static"),
            Ok(false),
            "unsupported {target}"
        );
        assert_eq!(
            build_support::native_target(false, "other-host", target, "crt-static"),
            Ok(false),
            "disabled {target}"
        );
    }
}

#[test]
fn native_linux_jit_requires_matching_host_and_target() {
    for target in ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"] {
        assert_eq!(
            build_support::native_target(true, target, target, "sse,sse2"),
            Ok(true),
            "native {target}"
        );
        assert_eq!(
            build_support::native_target(false, "other-host", target, "crt-static"),
            Ok(false),
            "interpreter cross-build"
        );
        assert!(
            build_support::native_target(true, "other-host", target, "").is_err(),
            "JIT cross-build rejected"
        );
    }
}

#[test]
fn native_jit_rejects_static_crt_for_gnu_and_musl() {
    for target in ["x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"] {
        for features in ["crt-static", "crt-static,sse,sse2", "sse,crt-static,sse2"] {
            assert!(matches!(
                build_support::native_target(true, target, target, features),
                Err(error) if error.contains("requires a dynamic CRT")
                    && error.contains("target-feature=-crt-static")
            ));
        }
        assert_eq!(
            build_support::native_target(true, target, target, ""),
            Ok(true),
            "dynamic CRT accepted for {target}"
        );
    }
}
