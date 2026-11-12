// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
*/
import "C"

func emitShift(code []byte) ([]byte, error) {
	if len(code) != 2 {
		return nil, errInstruction
	}
	var e emitter
	target, source := code[1]&15, code[1]>>4
	if target > 12 || (code[0]&1 != 0 && source >= 8) {
		e.trap(0)
		return e, nil
	}
	word := target >= 8
	e.loadReg(target)
	if code[0]&1 == 0 {
		e.bytes(0xB9, source, 0, 0, 0)
	} else {
		e.bytes(0x0F, 0xB6, 0x4F, C.R8_OFF_regs+source)
	}
	width := byte(8)
	if word {
		width = 16
	}
	e.bytes(0x83, 0xF9, 1)
	lower := e.branch(0x83)
	e.bytes(0xB9, 1, 0, 0, 0)
	e.patch(lower)
	e.bytes(0x83, 0xF9, width)
	upper := e.branch(0x86)
	e.bytes(0xB9, width, 0, 0, 0)
	e.patch(upper)
	group := byte(4)
	switch code[0] & 0xFE {
	case 0xC2:
		group = 5
	case 0xC4:
		group = 7
	case 0xC6:
		group = 2
	case 0xC8:
		group = 3
	}
	rotate := code[0] >= 0xC6
	if rotate {
		e.bytes(0x0F, 0xB6, 0x57, C.R8_OFF_flags, 0x0F, 0xBA, 0xE2, 0)
	} else {
		e.bytes(0xFF, 0xC9)
	}
	// Split shifts into count-1 and 1 so carry is defined at the operand width.
	if word {
		e.bytes(0x66, 0xD3)
	} else {
		e.bytes(0xD2)
	}
	e.bytes(0xC0 | group<<3)
	if !rotate {
		if word {
			e.bytes(0x66, 0xD1)
		} else {
			e.bytes(0xD0)
		}
		e.bytes(0xC0 | group<<3)
	}
	e.storeReg(target)
	e.flags(flagNZC, 0x92, word)
	e.finish()
	return e, nil
}
