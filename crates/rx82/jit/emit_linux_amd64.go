// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
*/
import "C"

type emitter []byte

const (
	flagNZ   byte = 0x82
	flagNZC  byte = 0x83
	flagNZCV byte = 0xC3
)

func (e *emitter) bytes(values ...byte)            { *e = append(*e, values...) }
func (e *emitter) word(value uint16)               { e.bytes(byte(value), byte(value>>8)) }
func (e *emitter) set8(offset byte, value byte)    { e.bytes(0xC6, 0x47, offset, value) }
func (e *emitter) set16(offset byte, value uint16) { e.bytes(0x66, 0xC7, 0x47, offset); e.word(value) }
func (e *emitter) finish()                         { e.set16(C.R8_OFF_state, C.R8_FETCH_OPCODE); e.bytes(0xC3) }

// flags takes N/Z from AL for bytes or AX for words. Capture any requested host
// carry and overflow before TEST, then merge them with preserved guest flags.
func (e *emitter) flags(changed byte, carryCondition byte, word bool) {
	if changed&1 != 0 {
		e.bytes(0x0F, carryCondition, 0xC1)
	}
	if changed&0x40 != 0 {
		e.bytes(0x41, 0x0F, 0x90, 0xC0)
	} // seto r8b
	if word {
		e.bytes(0x66, 0x85, 0xC0)
	} else {
		e.bytes(0x84, 0xC0)
	}
	e.bytes(0x41, 0x0F, 0x94, 0xC1, 0x41, 0x0F, 0x98, 0xC2) // setz r9b; sets r10b
	e.bytes(0x0F, 0xB6, 0x57, C.R8_OFF_flags, 0x83, 0xE2)
	e.bytes(^changed)
	if changed&1 != 0 {
		e.bytes(0x0F, 0xB6, 0xC9, 0x09, 0xCA) // movzx ecx,cl; or edx,ecx
	}
	if changed&0x40 != 0 {
		e.bytes(0x45, 0x0F, 0xB6, 0xC0, 0x41, 0xC1, 0xE0, 6, 0x44, 0x09, 0xC2)
	}
	e.bytes(0x45, 0x0F, 0xB6, 0xC9, 0x41, 0xD1, 0xE1, 0x44, 0x09, 0xCA)
	e.bytes(0x45, 0x0F, 0xB6, 0xD2, 0x41, 0xC1, 0xE2, 7, 0x44, 0x09, 0xD2)
	e.bytes(0x88, 0x57, C.R8_OFF_flags) // mov [rdi+flags],dl
}

// pushAL requests a stack write and decrements SP with 16-bit wraparound.
func (e *emitter) pushAL() {
	e.bytes(0x88, 0x47, C.R8_OFF_request_data, 0x0F, 0xB7, 0x57, C.R8_OFF_sp)
	e.bytes(0x66, 0x89, 0x57, C.R8_OFF_request_addr, 0x66, 0xFF, 0xCA, 0x66, 0x89, 0x57, C.R8_OFF_sp)
	e.set8(C.R8_OFF_request, 4)
}

func emitExecute(code []byte) ([]byte, error) {
	var e emitter
	if len(code) == 0 {
		return nil, errInstruction
	}
	if reg := code[0] & 15; code[0] >= 0x10 && code[0] < 0xC0 && code[0]&0xF0 != 0x20 && reg >= 8 && reg <= 12 {
		return emitWord(code)
	}
	switch op := code[0]; {
	case op == 0x01 && len(code) == 1:
		e.finish()
	case op >= 0x10 && op <= 0x17 && len(code) == 2:
		e.bytes(0xB0, code[1], 0x88, 0x47, op-0x10)
		e.flags(flagNZ, 0, false)
		e.finish()
	case op >= 0x50 && op <= 0x57 && len(code) == 2:
		e.bytes(0x0F, 0xB6, 0x47, op-0x50, 0x0F, 0xB6, 0x57, C.R8_OFF_flags)
		e.bytes(0x0F, 0xBA, 0xE2, 0, 0x14, code[1], 0x88, 0x47, op-0x50) // bt edx,0; adc al,imm
		e.flags(flagNZCV, 0x92, false)
		e.finish()
	case (op&0xF8 == 0x30 || op&0xF8 == 0x40) && len(code) == 1:
		reg := op & 7
		e.bytes(0x0F, 0xB6, 0x47, reg, 0xFE)
		if op&0xF0 == 0x30 {
			e.bytes(0xC0)
		} else {
			e.bytes(0xC8)
		}
		e.bytes(0x88, 0x47, reg)
		e.flags(flagNZ, 0, false)
		e.finish()
	case op&0xF8 == 0x60 && len(code) == 2:
		reg := op & 7
		e.bytes(0x0F, 0xB6, 0x47, reg, 0x0F, 0xB6, 0x57, C.R8_OFF_flags)
		e.bytes(0x0F, 0xBA, 0xE2, 0, 0xF5, 0x1C, code[1], 0x88, 0x47, reg)
		e.flags(flagNZCV, 0x93, false)
		e.finish()
	case op&0xF8 == 0x70 && len(code) == 2:
		e.bytes(0x0F, 0xB6, 0x47, op&7, 0x2C, code[1])
		e.flags(flagNZC, 0x93, false)
		e.finish()
	case op&0xF8 == 0x80 || op&0xF8 == 0x90 || op&0xF8 == 0xA0 || op&0xF8 == 0xB0:
		if len(code) != 2 {
			return nil, errInstruction
		}
		reg := op & 7
		e.bytes(0x0F, 0xB6, 0x47, reg)
		switch op & 0xF0 {
		case 0x80, 0x90:
			e.bytes(0x24, code[1])
		case 0xA0:
			e.bytes(0x0C, code[1])
		case 0xB0:
			e.bytes(0x34, code[1])
		}
		if op&0xF0 != 0x90 {
			e.bytes(0x88, 0x47, reg)
		}
		e.flags(flagNZ, 0, false)
		e.finish()
	case (op == 0xF0 || op == 0xF2) && len(code) == 2:
		jump := -1
		if op == 0xF2 {
			e.bytes(0xF6, 0x47, C.R8_OFF_flags, 2, 0x75, 0)
			jump = len(e) - 1
		}
		e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_pc, 0x66, 0x05)
		e.word(uint16(int16(int8(code[1]))))
		e.bytes(0x66, 0x89, 0x47, C.R8_OFF_pc)
		if jump >= 0 {
			e[jump] = byte(len(e) - jump - 1)
		}
		e.finish()
	case op == 0xF8 && len(code) == 3:
		e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_pc, 0x88, 0x47, C.R8_OFF_arg0, 0xC1, 0xE8, 8)
		e.pushAL()
		e.set16(C.R8_OFF_arg16, uint16(code[1])|uint16(code[2])<<8)
		e.next(C.R8_WAIT_CALL)
	default:
		return emitOther(code)
	}
	return e, nil
}
