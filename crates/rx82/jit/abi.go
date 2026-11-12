// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
*/
import "C"

import (
	"errors"
	"fmt"
	"unsafe"
)

const abiVersion = C.R8JIT_ABI_VERSION

var errArgument = errors.New("invalid ABI argument")

// finish also contains Go panics. Caller buffers must meet the C header contract.
func finish(status *C.int32_t, diagnostic *C.char, capacity C.uint32_t, err *error) {
	if failure := recover(); failure != nil {
		*err = fmt.Errorf("compiler panic: %v", failure)
	}
	*status = C.R8JIT_OK
	switch {
	case errors.Is(*err, errVersion):
		*status = C.R8JIT_VERSION
	case errors.Is(*err, errHandle):
		*status = C.R8JIT_HANDLE
	case errors.Is(*err, errInstruction):
		*status = C.R8JIT_INSTRUCTION
	case errors.Is(*err, errBudget):
		*status = C.R8JIT_BUDGET
	case errors.Is(*err, errArgument):
		*status = C.R8JIT_ARGUMENT
	case *err != nil:
		*status = C.R8JIT_INTERNAL
	}
	if diagnostic != nil && capacity > 0 {
		buffer := unsafe.Slice((*byte)(unsafe.Pointer(diagnostic)), int(capacity))
		message := ""
		if *err != nil {
			message = (*err).Error()
		}
		n := copy(buffer[:len(buffer)-1], message)
		buffer[n] = 0
	}
}

//export r8jit_create
func r8jit_create(version C.uint32_t, budget C.uint64_t, output *C.uint64_t, diagnostic *C.char, capacity C.uint32_t) (status C.int32_t) {
	var err error
	defer finish(&status, diagnostic, capacity, &err)
	if output == nil {
		err = errArgument
		return
	}
	*output = 0
	var handle uint64
	handle, err = createCompiler(uint32(version), uint64(budget))
	if err == nil {
		*output = C.uint64_t(handle)
	}
	return
}

//export r8jit_compile
func r8jit_compile(handle C.uint64_t, code *C.uint8_t, length C.uint32_t, entry *C.uintptr_t, allocated *C.uint64_t, diagnostic *C.char, capacity C.uint32_t) (status C.int32_t) {
	var err error
	defer finish(&status, diagnostic, capacity, &err)
	if entry != nil {
		*entry = 0
	}
	if allocated != nil {
		*allocated = 0
	}
	if entry == nil || allocated == nil {
		err = errArgument
		return
	}
	if code == nil || length == 0 || length > 3 {
		err = errInstruction
		return
	}
	var result block
	result, err = compileInstruction(uint64(handle), C.GoBytes(unsafe.Pointer(code), C.int(length)))
	if err == nil {
		*entry = C.uintptr_t(result.entry)
		*allocated = C.uint64_t(result.size)
	}
	return
}

//export r8jit_invalidate
func r8jit_invalidate(handle C.uint64_t, diagnostic *C.char, capacity C.uint32_t) (status C.int32_t) {
	var err error
	defer finish(&status, diagnostic, capacity, &err)
	err = invalidateCompiler(uint64(handle))
	return
}

//export r8jit_destroy
func r8jit_destroy(handle C.uint64_t, diagnostic *C.char, capacity C.uint32_t) (status C.int32_t) {
	var err error
	defer finish(&status, diagnostic, capacity, &err)
	err = destroyCompiler(uint64(handle))
	return
}

func contextLayout() []uintptr {
	return []uintptr{C.sizeof_r8_context, C.R8_CONTEXT_ALIGN, C.R8_OFF_regs, C.R8_OFF_sp, C.R8_OFF_pc, C.R8_OFF_flags, C.R8_OFF_halt, C.R8_OFF_ins, C.R8_OFF_op_lo, C.R8_OFF_op_hi, C.R8_OFF_state, C.R8_OFF_arg16, C.R8_OFF_arg0, C.R8_OFF_arg1, C.R8_OFF_vendor, C.R8_OFF_capabilities, C.R8_OFF_extended, C.R8_OFF_family, C.R8_OFF_highest, C.R8_OFF_model, C.R8_OFF_revision, C.R8_OFF_bus_data, C.R8_OFF_request, C.R8_OFF_request_addr, C.R8_OFF_request_data, C.R8_OFF_host_output}
}

func main() {}

//export r8jit_release
func r8jit_release(handle C.uint64_t, entry C.uintptr_t) (status C.int32_t) {
	var err error
	defer finish(&status, nil, 0, &err)
	err = releaseEntry(uint64(handle), uintptr(entry))
	return
}

//export r8jit_compile_region
func r8jit_compile_region(handle C.uint64_t, pc C.uint16_t, code *C.uint8_t, length C.uint32_t, entry *C.uintptr_t, allocated *C.uint64_t, consumed *C.uint32_t) (status C.int32_t) {
	var err error
	defer finish(&status, nil, 0, &err)
	if entry != nil {
		*entry = 0
	}
	if allocated != nil {
		*allocated = 0
	}
	if consumed != nil {
		*consumed = 0
	}
	if entry == nil || allocated == nil || consumed == nil || code == nil || length == 0 || length > 256 {
		err = errArgument
		return
	}
	native, n, failure := emitRegion(uint16(pc), C.GoBytes(unsafe.Pointer(code), C.int(length)))
	if failure != nil {
		err = failure
		return
	}
	compilers.Lock()
	defer compilers.Unlock()
	instance := compilers.instances[uint64(handle)]
	if instance == nil {
		err = errHandle
		return
	}
	var result block
	result, err = instance.publish(native)
	if err == nil {
		*entry = C.uintptr_t(result.entry)
		*allocated = C.uint64_t(result.size)
		*consumed = C.uint32_t(n)
	}
	return
}
