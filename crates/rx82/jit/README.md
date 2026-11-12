# R8 native compiler

Go 1.27 compiles specialised R8 instructions into Linux amd64 System V code. Rust executes that code directly and retains the existing scheduler. The JIT is built and selected in the CLI by default on supported hosts. Turbo mode is enabled by default, including ROM boot, and uses bounded native regions. Use `--no-turbo` for execution paced to the nominal 4 MHz clock. The existing `--turbo` and `-t` options remain supported. Use `--backend interpreter` to select the reference backend, or build with `--no-default-features` to omit the JIT toolchain.

## Build and checks

Install Go 1.27, a native C compiler, stable Rust and libclang development libraries. Rust bindings are generated from `r8jit.h` using the optional `bindgen` build dependency. Interpreter builds do not invoke Go, the C compiler or libclang. Run all command examples from the repository root.

```sh
rtk cargo test -p rx82 --features jit
rtk cargo run -p rx82 --features jit -- --backend jit program.bin
rtk cargo test -p rx82 --no-default-features
(
    cd crates/rx82/jit || exit 1
    rtk go test ./...
    rtk go vet ./...
    rtk go test -race ./...
)
```

Use a writable `GOCACHE` if the default Go cache is restricted. Native builds require Cargo `HOST=TARGET`, using the GNU or musl Linux amd64 ABI with a dynamic CRT. Static-CRT JIT builds are rejected before Go runs. Musl defaults to a static CRT; set `RUSTFLAGS='-C target-feature=-crt-static'` for a native musl JIT build. Musl runtime execution has not been verified. Other targets retain the interpreter, including `--all-features` builds. Go builds a C archive in Cargo's output directory and Rust links it into the executable. Cargo installations need no separate JIT library and remain usable after their build directory is removed. System runtime libraries are still required.

## Local performance builds

Cargo builds compile the C glue with `-Ofast -march=native -mtune=native` by default. Set `R8JIT_NATIVE=0` to disable these flags. Existing `CGO_CFLAGS` are appended and can override the optimisation level. Go optimisations remain enabled by default. Set `GOAMD64` explicitly for the intended CPU; `v3` requires an x86-64-v3 host. These settings do not change the x64 instructions emitted by the JIT.

```sh
rtk proxy env GOAMD64=v3 cargo build --release -p rx82 --features jit
```

For Go PGO, collect a compilation profile from the Go module, then supply its absolute path to Cargo:

```sh
(
    cd crates/rx82/jit || exit 1
    rtk proxy env GOAMD64=v3 CGO_CFLAGS='-Ofast -march=native -mtune=native' go test -run '^$' -bench BenchmarkCompileCorpus -benchtime=10s -cpuprofile=/tmp/r8jit.pprof -o /tmp/r8jit-profile.test
)
rtk proxy env GOAMD64=v3 R8JIT_PGO=/tmp/r8jit.pprof cargo build --release -p rx82 --features jit
```

The corpus covers compilation, publication and reclamation for all 211 supported opcodes. It profiles compiler work rather than guest execution. Cargo tracks the profile and tuning environment for rebuilds. Without `R8JIT_PGO`, Go automatically uses `default.pgo` from the Go module when present. No profile is checked in. Profiles placed in the module directory are included in source packages. Compare repeated measurements after compatibility checks to assess any speedup.

Native builds require the build host’s instruction set. For portable distribution, use `tools/package_jit.py` as described in the [packaging instructions](#packaging-and-independent-checks). It replaces inherited CPU tuning, including Rust compiler flags.

## Packaging and independent checks

The Go module and C header live in `crates/rx82/jit/` and are included in the RX82 Cargo package. Run `rtk proxy python3 tools/test_cargo_jit.py` on native Linux amd64 to build the extracted Cargo distribution, install it, remove the build directory and run the installed JIT. It also verifies early static-CRT rejection. This check needs cached Cargo dependencies or an initial online dependency fetch.

From the repository root, run `rtk proxy python3 tools/package_jit.py`. It creates `dist/rx82-linux-amd64/` and refuses to overwrite an existing directory. Public packages enforce `R8JIT_NATIVE=0`, `GOAMD64=v1`, baseline x86-64 C and Rust settings, and discard custom tuning flags and `R8JIT_PGO`. The same settings apply to the offline source rebuild and are recorded in `package.json`. Local Cargo builds retain native tuning by default.

The package contains the executable, licence notices, dependency metadata and a source archive with vendored Rust dependencies. Packaging copies the executable into an otherwise empty directory and runs it without loader overrides. It also rebuilds and runs the source snapshot offline with an empty Cargo home. Run `rtk proxy python3 tools/test_package_jit.py` to check portable environment selection.

Packaging recognises rustup's `COPYRIGHT*.html` and `licenses/` layout, retaining its licence texts and relative paths. It also accepts flat `COPYRIGHT`, `LICENSE-MIT` and `LICENSE-APACHE` files. If your toolchain omits its notice files, supply `--go-license /path/to/Go/LICENSE` or `--rust-notices /path/to/rust/notices`. Python 3.11 or later and Git are needed for packaging. The packaged emulator needs neither Go nor libclang.

Run the full ROM and supplied-program corpus with `rtk cargo test --release -p rx82 --features jit --test jit_acceptance -- --ignored`. `tools/check_references.py` compares pinned external checkouts; its `--gr8`, `--go-r8` and `--typescript` options identify those directories. TypeScript checks need that reference's locked npm dependencies. These comparisons cover each external implementation's supported subset, not RX82 bus timing.

## Execution and compatibility

`System::default()` uses the interpreter for reference comparisons; the CLI selects the JIT explicitly. `System::tick` preserves exact cycle stepping, with opcode fetch and decode in Rust. Missing fetch history or changed specialisation assumptions fall back for the current cycle.

`System::run_cycles` and turbo-mode `System::run` use bounded native regions, including instruction fetch and decode, with direct forward and backward links. Calls retire at most 4,096 cycles. Devices must declare passive ROM semantics before regions are admitted. Debugging, state tracing and pre-existing bus writes retain exact stepping. Continuous monitor execution and `G` use this scheduler; `S` steps one instruction.

All 211 valid opcode bytes have native execution tests. Execution-time illegal operands trap natively. Illegal opcodes and their trap path remain in Rust. Native counters distinguish execution from fallback. CPU identification fields remain read-only native inputs; regions publish their last decoded instruction. Console traps return one output event for Rust to apply before devices tick.

Byte RAM transfers use guarded pointers and backing lengths. ROM overlaps or out-of-range accesses resume through the cycle backend. Stores become visible on the next fetch cycle. A store into compiled source exits before that fetch; source bytes are checked before region re-entry.

Up to four register pairs stay in callee-saved host registers. Pairs touched through byte aliases retain context access. Word negative and zero flags can remain deferred until a branch or exit needs them. INC/DEC pair loops followed by BNE use a fused host zero branch, with exact instruction execution for the remaining budget.

The cycle cache holds at most 64 entries and 256 KiB of executable mappings. Regions have a separate 64-entry, 1 MiB limit and evict least recently used entries under entry or byte pressure. Both caches release individual entries. Memory budgets charge whole mapped pages.

The compiler returns integer handles and diagnostics. Published pages are RX. Invalidation and destruction require execution to be stopped and invalidate all entries; reset clears cached code. If publication and release both fail, the compiler retains the RW mapping for later reclamation. The C header specifies buffer ownership and layout.

The Rust oracle panics for mixed-width operands in register SUB, AND, TEST, OR and XOR. Native execution produces an illegal-operand trap for those 400 encodings. Literal tests check the nine-cycle trap and stack frame; differential sweeps exclude only those panic cases. This choice follows RX82's width checks for LOAD, ADD and CMP; it has not been checked against every external implementation.

The release corpus covers ROM boot, three functional programs and four assembler examples through both cycle stepping and bounded execution. External comparisons cover 4,112 cases across the overlapping subsets of gr8, Jeremy Grecu's go-r8 and the TypeScript emulator. Their pinned revisions are in `tools/check_references.py`. They provide no independent RX82 device-timing reference. Native runtime checks cover Linux amd64; other host platforms remain unverified.

## Licence

Copyright (c) 2026 [Zayn Otley](https://ZaynOtley.com). [IntuitionEngine](https://IntuitionEngine.io).

The JIT compiler and this guide are licensed under the GNU Affero General Public License, version 3 or later (`AGPL-3.0-or-later`).

New JIT implementation and integration files carry `AGPL-3.0-or-later` notices. See `../LICENSES/AGPL-3.0-or-later.txt` and `../LICENSING.md`, also included in the Cargo package. Existing third-party source retains its original grants and notices. No IE implementation has been copied.

## Benchmarks

After the compatibility checks pass, run the workload benchmark from the repository root:

```sh
rtk cargo test --release -p rx82 --features jit --test jit_regions bounded_workload_performance -- --ignored --nocapture
```

The `states` feature must be disabled. `--all-features` enables state tracing and disables native regions. To pin the run on Linux, replace `rtk cargo` with `rtk proxy taskset -c 2 cargo`, substituting an allowed CPU number for `2`.

Five samples on 12 November 2026 used a Xeon W-11955M running Linux amd64,
pinned to CPU 2, with Go 1.27.3, Rust 1.99.0, Cargo release defaults, native C
tuning and no PGO profile. Execution used 4,096-cycle chunks in turbo mode with
state tracing disabled. CPU, bus, RAM and cycle counts matched the interpreter.

| Workload | Cycles | Interpreter median | Cold JIT median | Warm JIT median |
| --- | --- | --- | --- | --- |
| countdown | 7,209,058 | 136.831 ms | 1.356 ms | 1.236 ms |
| memcpy | 11,258 | 264.747 us | 37.601 us | 3.663 us |

Cold timing includes compilation. Warm timing restores CPU, bus and RAM while
retaining compiled code. Compiler instance creation is outside both timers.
In a repository checkout, raw samples and the source hash are in
[`R8_X64_JIT_PERFORMANCE.json`](../../../R8_X64_JIT_PERFORMANCE.json).
This JSON file is omitted from the standalone Cargo package.
These results predate the archive-linking and monitor scheduler fixes. CPU
frequency and competing host activity were not controlled. The results apply to
these admitted turbo workloads; debugging, custom devices and clock pacing use
exact stepping.
