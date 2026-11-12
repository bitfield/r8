// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
*/
import "C"

func (e *emitter) load16(reg byte) {
	if reg == 12 {
		e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_sp)
		return
	}
	e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_regs+(reg-8)*2, 0x66, 0xC1, 0xC0, 8)
}

// Byte stores preserve host flags for later publication. R8 pairs are big endian.
func (e *emitter) store16(reg byte) {
	if reg == 12 {
		e.bytes(0x66, 0x89, 0x47, C.R8_OFF_sp)
		return
	}
	offset := C.R8_OFF_regs + (reg-8)*2
	e.bytes(0x88, 0x67, offset, 0x88, 0x47, offset+1)
}

func emitWord(code []byte) ([]byte, error) {
	return emitWordCached(code, nil, false)
}

func emitWordCached(code []byte, cache *wordCache, deferNZ bool) ([]byte, error) {
	op, reg := code[0]&0xF0, code[0]&15
	unary := op == 0x30 || op == 0x40
	if (unary && len(code) != 1) || (!unary && len(code) != 3) {
		return nil, errInstruction
	}
	var e emitter
	changed, carryCondition := flagNZ, byte(0)
	if op != 0x10 {
		cache.load(&e, reg)
	}
	switch op {
	case 0x10:
		e.bytes(0xB8, code[1], code[2], 0, 0)
	case 0x30:
		e.bytes(0x66, 0xFF, 0xC0)
	case 0x40:
		e.bytes(0x66, 0xFF, 0xC8)
	case 0x50, 0x60:
		e.bytes(0x0F, 0xB6, 0x57, C.R8_OFF_flags, 0x0F, 0xBA, 0xE2, 0)
		instruction := byte(0x15)
		carryCondition = 0x92
		if op == 0x60 {
			e.bytes(0xF5)
			instruction = 0x1D
			carryCondition = 0x93
		}
		e.bytes(0x66, instruction, code[1], code[2])
		changed = flagNZCV
	case 0x70:
		e.bytes(0x66, 0x2D, code[1], code[2])
		changed, carryCondition = flagNZC, 0x93
	case 0x80, 0x90:
		e.bytes(0x66, 0x25, code[1], code[2])
	case 0xA0:
		e.bytes(0x66, 0x0D, code[1], code[2])
	case 0xB0:
		e.bytes(0x66, 0x35, code[1], code[2])
	default:
		return nil, errInstruction
	}
	if op != 0x70 && op != 0x90 {
		cache.store(&e, reg)
	}
	if !deferNZ {
		e.flags(changed, carryCondition, true)
	}
	e.finish()
	return e, nil
}
