[![Crate](https://img.shields.io/crates/v/r8asm.svg)](https://crates.io/crates/r8asm)
[![Docs](https://docs.rs/r8asm/badge.svg)](https://docs.rs/r8asm)
![CI](https://github.com/bitfield/r8/actions/workflows/ci.yml/badge.svg)
![Audit](https://github.com/bitfield/r8/actions/workflows/audit.yml/badge.svg)
![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

An assembler for the R8 fantasy retro CPU architecture.

# Installation

```sh
cargo install --locked r8asm
```

# Usage

## Assembling R8 source files

Prepare your program in a text file, and run:

```sh
r8asm my_prog.asm
```

If the program assembles correctly, this will produce a `my_prog.bin` file you can run with the monitor.

Use the `--list` switch to also produce a listing to the console:

```
r8asm --list my_prog.asm
```

```
C000              RAM_TEST:
C000 19 FD FF         ld cd, 0xFFFD  ; top of possible RAM
C003 10 02            ld a, 0x02
```

## Disassembling R8 binary files

Run:

```sh
r8asm -d my_prog.bin
```

This will print the disassembled listing.

# See also

* [`r8cpu`](https://crates.io/crates/r8cpu): Core types and logic for the R8 architecture.
* [`rx82`](https://crates.io/crates/rx82): A low-level emulator for an R8-based computer system.
