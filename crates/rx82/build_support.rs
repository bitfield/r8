// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg_attr(
    not(test),
    expect(clippy::single_call_fn, reason = "shared with target policy tests")
)]
pub fn native_target(
    enabled: bool,
    host: &str,
    target: &str,
    target_features: &str,
) -> Result<bool, &'static str> {
    if !enabled
        || !matches!(
            target,
            "x86_64-unknown-linux-gnu" | "x86_64-unknown-linux-musl"
        )
    {
        return Ok(false);
    }
    if host != target {
        return Err(
            "the Linux amd64 JIT requires HOST=TARGET; disable the jit feature for cross-builds",
        );
    }
    if target_features
        .split(',')
        .any(|feature| feature == "crt-static")
    {
        return Err(
            "the Linux amd64 JIT requires a dynamic CRT; set RUSTFLAGS='-C target-feature=-crt-static' or disable the jit feature",
        );
    }
    Ok(true)
}
