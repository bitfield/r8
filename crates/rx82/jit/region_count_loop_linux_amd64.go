// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
package main

/*
#include "r8jit.h"
*/
import "C"

func countLoop(code []byte, cache *wordCache) bool {
	if len(code) < 3 || code[1] != 0xF2 || code[2] != 0xFD {
		return false
	}
	op, reg := code[0]&0xF0, code[0]&15
	return (op == 0x30 || op == 0x40) && cache.slot(reg) >= 0
}

// The common INC/DEC pair loop uses the host zero condition directly. A short
// remaining budget resumes the ordinary instruction path, which preserves
// intermediate CPU and bus state.
func (e *emitter) fusedCountLoop(op byte, address uint16, cache *wordCache) int {
	reg := op & 15
	e.bytes(0x41, 0x83, 0xFC, 10)
	initialRemainder := e.branch(0x82)
	e.commitRAMWrite()
	first := e.relativeJump()
	head := len(*e)
	e.bytes(0x41, 0x83, 0xFC, 10)
	enough := e.branch(0x83)
	e.publishWordNZ(cache, int(reg))
	remainder := e.relativeJump()
	e.patch(first)
	e.patch(enough)
	e.bytes(0x41, 0x83, 0xEC, 10, 0x41, 0x83, 0xC3, 10)
	cache.load(e, reg)
	operation := byte(0xC8)
	if op&0xF0 == 0x30 {
		operation = 0xC0
	}
	e.bytes(0x66, 0xFF, operation)
	cache.store(e, reg)
	e.set16(C.R8_OFF_pc, address+3)
	e.set8(C.R8_OFF_ins, 0xF2)
	e.set8(C.R8_OFF_op_lo, 0xFD)
	e.set8(C.R8_OFF_op_hi, 0)
	e.set8(C.R8_OFF_bus_data, 0xFD)
	e.set16(C.R8_REGION_ADDR, address+2)
	e.set8(C.R8_REGION_MEM, 1)
	e.set8(C.R8_REGION_WRITE, 0)
	done := e.branch(0x84)
	e.set16(C.R8_OFF_pc, address)
	jump := e.relativeJump()
	e.patchTo(jump, head)
	e.patch(done)
	e.publishWordNZ(cache, int(reg))
	exit := e.relativeJump()
	e.patch(initialRemainder)
	e.patch(remainder)
	return exit
}
