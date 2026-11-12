// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
int32_t r8_probe_entry(uintptr_t entry, r8_context *context);
*/
import "C"

import "unsafe"

// These ABI probes live outside _test.go files because cgo requires it.
func probeCreate(version uint32, capacity int) (int32, uint64, string, byte) {
	buffer := make([]byte, capacity+1)
	buffer[capacity] = 0x5A
	handle := C.uint64_t(99)
	status := r8jit_create(C.uint32_t(version), 4096, &handle, (*C.char)(unsafe.Pointer(&buffer[0])), C.uint32_t(capacity))
	end := 0
	for end < capacity && buffer[end] != 0 {
		end++
	}
	return int32(status), uint64(handle), string(buffer[:end]), buffer[capacity]
}

func probeNative(entry uintptr, _ bool) (int32, uint16, uint8) {
	context := C.r8_context{state: C.R8_EXECUTE}
	status := C.r8_probe_entry(C.uintptr_t(entry), &context)
	return int32(status), uint16(context.state), uint8(context.arg0)
}

func probeALU(entry uintptr, reg, value, flags byte) (byte, byte, uint16, int32) {
	context := C.r8_context{state: C.R8_EXECUTE, flags: C.uint8_t(flags)}
	context.regs[reg] = C.uint8_t(value)
	status := C.r8_probe_entry(C.uintptr_t(entry), &context)
	return byte(context.regs[reg]), byte(context.flags), uint16(context.state), int32(status)
}

func probeCompile(handle uint64, code []byte) (int32, uintptr, uint64, string) {
	entry, allocated := C.uintptr_t(99), C.uint64_t(99)
	var pointer *C.uint8_t
	if len(code) > 0 {
		pointer = (*C.uint8_t)(unsafe.Pointer(&code[0]))
	}
	var diagnostic [256]C.char
	status := r8jit_compile(C.uint64_t(handle), pointer, C.uint32_t(len(code)), &entry, &allocated, &diagnostic[0], 256)
	return int32(status), uintptr(entry), uint64(allocated), C.GoString(&diagnostic[0])
}

func probeRegion(entry uintptr, budget uint32) (int32, uint32, [8]byte, uint16) {
	context := C.r8_region{budget: C.uint32_t(budget)}
	context.cpu.state = C.R8_FETCH_OPCODE
	status := C.r8_probe_entry(C.uintptr_t(entry), &context.cpu)
	var regs [8]byte
	for i := range regs {
		regs[i] = byte(context.cpu.regs[i])
	}
	return int32(status), uint32(context.cycles), regs, uint16(context.cpu.sp)
}
