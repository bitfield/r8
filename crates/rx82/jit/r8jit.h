/* SPDX-FileCopyrightText: 2026 Zayn Otley
 * https://ZaynOtley.com
 * https://IntuitionEngine.io
 * SPDX-License-Identifier: AGPL-3.0-or-later */
#ifndef R8JIT_H
#define R8JIT_H

#include <stddef.h>
#include <stdint.h>

#define R8JIT_ABI_VERSION 1
#define R8JIT_OK 0
#define R8JIT_VERSION 1
#define R8JIT_HANDLE 2
#define R8JIT_INSTRUCTION 3
#define R8JIT_BUDGET 4
#define R8JIT_INTERNAL 5
#define R8JIT_ARGUMENT 6

/* Tags are independent of Rust enum layout. Ignore payload fields unused by the
 * current tag; they may retain values from an earlier state. */
enum {
    R8_DECODE, R8_EXECUTE, R8_FETCH_OPCODE, R8_PUSH_DATA, R8_PUSH_FLAGS,
    R8_PUSH_RET_LO, R8_PUSH_TRAP, R8_READ_DATA, R8_READ_DEC, R8_READ_FLAGS,
    R8_READ_INC, R8_READ_OP, R8_READ_OP_HI, R8_READ_OP_LO, R8_READ_PS,
    R8_READ_RET_LO, R8_READ_STACK_HI, R8_READ_VEC_HI, R8_READ_VEC_LO,
    R8_REQ_VEC_LO, R8_WAIT_CALL, R8_WAIT_DATA, R8_WAIT_DEC, R8_WAIT_FLAGS,
    R8_WAIT_INC, R8_WAIT_OP, R8_WAIT_OP_HI, R8_WAIT_OP_LO, R8_WAIT_OPCODE,
    R8_WAIT_PS, R8_WAIT_RET_LO, R8_WAIT_STACK_HI, R8_WAIT_VEC_HI, R8_WAIT_VEC_LO
};

/* Cycle entry: void entry(r8_context *). No callbacks.
 * Supply current CPU state and resolved bus data. At Execute, validate the
 * instruction and operands against the compiled bytes before entry.
 * Clear request, request_addr, request_data and host_output before each call.
 * A returned state of 0xFFFF requests fallback without retiring a CPU cycle:
 * discard context changes and execute the current cycle in Rust instead.
 * Otherwise, one CPU cycle has completed.
 * Registers are A..H; SP is separate. All boolean fields are 0 or 1.
 * arg16 holds address payloads, arg0 registers or bytes, and arg1 the trap code
 * for R8_PUSH_RET_LO.
 * Rust retains its pending bus request. request is 0 (none), 1 (disable),
 * 2 (read), 3 (write data), or 4 (write memory). Rust applies first-writer
 * arbitration after return. Compiler input is copied and never retained. */
typedef struct {
    uint8_t regs[8];
    uint16_t sp;
    uint16_t pc;
    uint8_t flags;
    uint8_t halt;
    uint8_t ins;
    uint8_t op_lo;
    uint8_t op_hi;
    uint8_t reserved0[3];
    uint16_t state;
    uint16_t arg16;
    uint8_t arg0;
    uint8_t arg1;
    uint8_t reserved1[2];
    uint8_t vendor[8];
    uint8_t capabilities;
    uint8_t extended;
    uint8_t family;
    uint8_t highest;
    uint8_t model;
    uint8_t revision;
    uint8_t reserved2[2];
    uint8_t bus_data;
    uint8_t request;
    uint16_t request_addr;
    uint8_t request_data;
    uint8_t host_output; /* 1 requests the RX82 console trap character output. */
    uint8_t reserved3[2];
} r8_context;

/* Region entry: void entry(r8_region *). The CPU must be unhalted at
 * R8_FETCH_OPCODE with the PC used for compilation. Validate source bytes before
 * entry. All attached devices must be passive ROM; the bus must have neither
 * an active write line nor a pending update. Clear bus_write and the CPU output
 * fields listed above before each call.
 * Non-zero ram_length requires exclusively borrowed writable backing for that
 * guest RAM range at ram_start, without overlap with ROM mappings.
 * Returns at an instruction boundary with cycles <= budget. When cycles is
 * non-zero, CPU and bus outputs are valid; otherwise ignore them and step in Rust. */
typedef struct {
    r8_context cpu;
    uint32_t budget;
    uint32_t cycles;
    uint16_t bus_addr;
    uint8_t bus_mem;
    uint8_t bus_write;
    uint8_t *ram; /* Borrowed only during native execution. */
    uint16_t ram_start;
    uint32_t ram_length; /* Zero disables direct RAM access. */
} r8_region;
enum {
    R8_REGION_BUDGET = offsetof(r8_region, budget),
    R8_REGION_CYCLES = offsetof(r8_region, cycles),
    R8_REGION_ADDR = offsetof(r8_region, bus_addr),
    R8_REGION_MEM = offsetof(r8_region, bus_mem),
    R8_REGION_WRITE = offsetof(r8_region, bus_write),
    R8_REGION_RAM = offsetof(r8_region, ram),
    R8_REGION_RAM_START = offsetof(r8_region, ram_start),
    R8_REGION_RAM_LENGTH = offsetof(r8_region, ram_length)
};
int32_t r8jit_compile_region(uint64_t handle, uint16_t pc, uint8_t *code,
    uint32_t length, uintptr_t *entry, uint64_t *allocated, uint32_t *consumed);

/* Call synchronously. An entry must not be executing when released; all entries
 * of an instance must be stopped during invalidation or destruction.
 * Entries remain valid until release, invalidation or destruction. Release
 * invalidates only its entry. Instances are independent.
 * Caller owns every output and diagnostic buffer; the capacity
 * describes writable bytes. Diagnostics are NUL-terminated when capacity > 0.
 * On failure outputs are zero. Successful calls clear diagnostics. */
int32_t r8jit_create(uint32_t version, uint64_t budget, uint64_t *handle,
                     char *diagnostic, uint32_t capacity);
int32_t r8jit_compile(uint64_t handle, uint8_t *code, uint32_t length,
                      uintptr_t *entry, uint64_t *allocated,
                      char *diagnostic, uint32_t capacity);
int32_t r8jit_release(uint64_t handle, uintptr_t entry);
int32_t r8jit_invalidate(uint64_t handle, char *diagnostic, uint32_t capacity);
int32_t r8jit_destroy(uint64_t handle, char *diagnostic, uint32_t capacity);

#define R8_OFFSET(field) enum { R8_OFF_##field = offsetof(r8_context, field) }
R8_OFFSET(regs); R8_OFFSET(sp); R8_OFFSET(pc); R8_OFFSET(flags);
R8_OFFSET(halt); R8_OFFSET(ins); R8_OFFSET(op_lo); R8_OFFSET(op_hi);
R8_OFFSET(state); R8_OFFSET(arg16); R8_OFFSET(arg0); R8_OFFSET(arg1);
R8_OFFSET(vendor); R8_OFFSET(capabilities); R8_OFFSET(extended);
R8_OFFSET(family); R8_OFFSET(highest); R8_OFFSET(model); R8_OFFSET(revision);
R8_OFFSET(bus_data); R8_OFFSET(request); R8_OFFSET(request_addr); R8_OFFSET(request_data);
R8_OFFSET(host_output);
#undef R8_OFFSET
enum { R8_CONTEXT_ALIGN = _Alignof(r8_context) };
#endif
