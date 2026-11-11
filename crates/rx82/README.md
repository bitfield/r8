[![Crate](https://img.shields.io/crates/v/rx82.svg)](https://crates.io/crates/rx82)
[![Docs](https://docs.rs/rx82/badge.svg)](https://docs.rs/rx82)
![CI](https://github.com/bitfield/r8/actions/workflows/ci.yml/badge.svg)
![Audit](https://github.com/bitfield/r8/actions/workflows/audit.yml/badge.svg)
![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

An emulator for the RX82 fantasy retro computer system, including the R8 8-bit CPU.

> ADRIC: _What do these numbers and letters mean?_\
> DOCTOR: _It's an early version. Instructions have to be punched in by machine code._\
> ADRIC: _Oh, how boring._\
> DOCTOR: _**Boring?**_\
—Doctor Who, _Logopolis_

![](img/RX82.jpg)

# Installation

```sh
cargo install --locked rx82
```

# About

This is an emulator for the RX82 architecture, an imagined home computer system similar to those of the early 1980s, such as the Sinclair ZX81 and Spectrum, the BBC Micro, or the Commodore 64.

The RX82's design is intended not only to evoke fond memories in those of a certain age, but also to help teach the fundamentals of computer systems architecture and computer engineering. It's simpler than historic systems such as the ZX81, because no cost or design compromises are required, but also realistic enough to be useful for learning purposes.

Its central processor is the R8, a fan-fiction CPU design comparable to the Zilog Z80 or the MOS 6502, but again, somewhat simplified for educational purposes.

This crate provides a reference implementation of the RX82 and R8 architectures, and an assembler / disassembler for use with R8 assembly language programs. However, it is intended to be modular, so that you can pick and choose components to build your own systems.

For example, you could use the R8 CPU as part of your own emulator that replaces the RX82 system with something else. Equally, you could use the RX82 system components but replace the CPU with a design of your own, or an emulated real machine such as a 6502.

# Usage

## Starting the monitor

To start the monitor in debug (single-step) mode:

```sh
rx82
```

```txt
(C) 1982 RX Computers Ltd.

0xBF00 bytes free. Ready.
```

You can also optionally load and run a binary file (such as one produced by the assembler, for example):

```sh
rx82 my_prog.bin
```

To run the binary in single-step mode, use the `--step` switch:

```sh
rx82 --step my_prog.bin
```

## Using the monitor

The monitor displays the current CPU registers and the next instruction in memory, then prompts for a command. Type `H` for help:

```txt
RMON v1.0 (C) 1976 Solid State Technologies, Inc.
  PC  A  B  C  D  E  F  G  H   SP ZC | NEXT
C00B 02 00 FF F6 00 80 00 00 BFFF 01 | bne 0xF8
> h
Commands:
G [<address>] = Go (run till halted)
H             = Help
M [<address>] = Memory dump
Q             = Quit
R             = Register display
S [<address>] = Single step
Enter         = Repeat last command
>
```

To assemble or disassemble R8 programs, use the standalone [`r8asm`](https://crates.io/crates/r8asm) tool.

# RX82 architecture

The RX82 is a single-board computer with one R8 CPU clocked at 4Mhz, 64KiB of static RAM, an 8-bit data bus, and a 16-bit address bus.

## Memory map

| Address | Contents |
| :--- | :---    |
| `0x0000` | Trap/interrupt table |
| `0x0080` | System data area |
| `0x0100` | User RAM |
| `0xC000` | ROM |
| `0xFF00` | System I/O area |
| `0xFFFE` | Reset vector |

## Boot process

At power on, the CPU loads the reset vector at `0xFFFE`, which in the RX82 system holds the ROM entry point, `0xC000`. Execution begins here and a simple RAM test is performed to find the highest writable address in memory. The stack pointer is initialised to this address.

The trap table is initialised, and all undefined traps are vectored to a single 'undefined trap' handler.

Finally, the interactive monitor is invoked.

## OS traps

The following general-purpose traps are defined:

| Code | Name | Purpose | Inputs |
| :--- | :--- | :--- | :--- |
| `0x20` | PUTCHAR | Print character to terminal | A = ASCII code of character |

## R8 technical information

The [R8 Programmer's Manual](https://bitfieldconsulting.com/books/r8-prog-manual) is the official documentation of the R8 architecture, instruction set, and assembly language.

You can find more resources, software, and information on the [R8 CPU](https://bitfieldconsulting.com/r8-cpu) page.

# About the emulator

This is a **cycle-stepped** emulator (sometimes called a “low-level” emulator) that models the whole computer system, including the CPU, devices, bus, and so forth. Unlike a “high-level”, or **instruction-stepped** emulator, where the CPU “owns” all the resources, such as memory, and can manipulate them directly, in a low-level emulator the CPU must read and write signals to the bus like any other device.

This makes it more complicated, since the emulator must model the CPU's internal state (fetch, decode, execute, and so on), the bus signalling, and all the devices, but it's also more realistic and interesting.

If you're interested in writing an emulator, though, it's much easier to get started with a high-level one. You can read a tutorial series on writing a high-level R8 emulator starting here:

* [Emergency in space: emulating a CPU](https://bitfieldconsulting.com/posts/emergency-space)

# See also

* [`r8asm`](https://crates.io/crates/r8asm): An R8 assembler / disassembler.
* [`r8cpu`](https://crates.io/crates/r8cpu): Core types and logic for the R8 architecture.

# Changelog

* **0.8.0** — remove subcommands (make assembler fully standalone) and auto-doc feature
* **0.7.0** — opcode complete: `asr`, `bmi`, `bpl`, `cli`, `cpuid`, `or`, `push ps`, `pop ps`, `rol`, `ror`, `sei`, `shl`, `test`, `trapv`, `xor`; refactor ALU functions into `r8cpu` crate; negative and overflow flags; all 16-bit modes implemented; autogenerate state diagram
* **0.6.0** — split into assembler / core / emulator crates, `lsr`, `and R, N`, `add R, N`, `sub R, N`, `jmp NN`, `bcc` / `bcs`, `sec / clc`, `include`, turbo mode, ROM improvements
* **0.5.0** — `org` and `data` directives, traps implemented, `trap`, `rti`, `call`, `ret`, `ld (RR), R`, `ld R1, R2`, `push`, `pop`, `inc/dec (RR)`, `inc/dec (NN)`, `bra` instructions, reset vector, stack pointer, ROM binary, forward labels
* **0.4.0** — `beq`, `bne`, `inc`, `dec`, and `cmp` instructions; zero and carry flags; backward labels, comments
* **0.3.0** — all registers, load immediate and store direct instructions
* **0.2.0** — monitor improvements, add `halt` instruction, add assembler
* **0.1.0** — first release
