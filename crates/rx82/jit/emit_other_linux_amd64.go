// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
*/
import "C"

func emitRegister(code []byte) ([]byte, error) {
	if len(code) != 2 {
		return nil, errInstruction
	}
	var e emitter
	source, target := code[1]>>4, code[1]&15
	if source > 12 || target > 12 || (source < 8) != (target < 8) {
		e.trap(0)
		return e, nil
	}
	word := target >= 8
	e.loadReg(source)
	if code[0] == 0x1E {
		e.storeReg(target)
		e.flags(flagNZ, 0, word)
		e.finish()
		return e, nil
	}
	e.bytes(0x89, 0xC6) // mov esi,eax
	e.loadReg(target)
	changed, carry, operation := flagNZ, byte(0), byte(0)
	switch code[0] {
	case 0x5F, 0x6F:
		e.bytes(0x0F, 0xB6, 0x57, C.R8_OFF_flags, 0x0F, 0xBA, 0xE2, 0)
		changed, carry, operation = flagNZC, 0x92, 0x10
		if code[0] == 0x6F {
			e.bytes(0xF5)
			carry, operation = 0x93, 0x18
		}
	case 0x7F:
		changed, carry, operation = flagNZC, 0x93, 0x28
	case 0x8F, 0x9F:
		operation = 0x20
	case 0xAF:
		operation = 0x08
	case 0xBF:
		operation = 0x30
	default:
		return nil, errInstruction
	}
	if word {
		e.bytes(0x66, operation+1, 0xF0)
	} else {
		e.bytes(0x40, operation, 0xF0)
	}
	if code[0] != 0x7F && code[0] != 0x9F {
		e.storeReg(target)
	}
	e.flags(changed, carry, word)
	e.finish()
	return e, nil
}

func emitOther(code []byte) ([]byte, error) {
	op := code[0]
	if op >= 0xC0 && op <= 0xC9 {
		return emitShift(code)
	}
	if op == 0x1E || (op >= 0x5F && op <= 0xBF && op&15 == 15) {
		return emitRegister(code)
	}
	var e emitter
	switch {
	case op == 0 && len(code) == 1:
		e.set8(C.R8_OFF_halt, 1)
	case (op == 3 || op == 4 || op == 5 || op == 6) && len(code) == 1:
		mask := byte(1)
		if op >= 5 {
			mask = 4
		}
		if op == 3 || op == 5 {
			e.bytes(0x80, 0x4F, C.R8_OFF_flags, mask)
		} else {
			e.bytes(0x80, 0x67, C.R8_OFF_flags, ^mask)
		}
	case op == 0xFA && len(code) == 2:
		field := byte(0)
		switch code[1] {
		case 0:
			field = C.R8_OFF_highest
		case 1:
			e.bytes(0x48, 0x8B, 0x47, C.R8_OFF_vendor, 0x48, 0x89, 0x47, C.R8_OFF_regs)
		case 2:
			e.copy8(C.R8_OFF_family, C.R8_OFF_regs)
			e.copy8(C.R8_OFF_model, C.R8_OFF_regs+1)
			e.copy8(C.R8_OFF_revision, C.R8_OFF_regs+2)
		case 3:
			field = C.R8_OFF_extended
		case 4:
			field = C.R8_OFF_capabilities
		}
		if field != 0 {
			e.bytes(0x0F, 0xB6, 0x47, field)
			e.store8(0)
			e.flags(flagNZ, 0, false)
		}
	case op >= 0xF1 && op <= 0xF6 && len(code) == 2:
		masks := [6]byte{2, 2, 1, 1, 0x80, 0x80}
		e.bytes(0xF6, 0x47, C.R8_OFF_flags, masks[op-0xF1])
		condition := byte(0x84)
		if op&1 == 0 {
			condition = 0x85
		}
		skip := e.branch(condition)
		e.bytes(0x66, 0x81, 0x47, C.R8_OFF_pc)
		e.word(uint16(int16(int8(code[1]))))
		e.patch(skip)
	case op == 0xF7 && len(code) == 3:
		e.set16(C.R8_OFF_pc, uint16(code[1])|uint16(code[2])<<8)
	case op >= 0x20 && op <= 0x27 && len(code) == 3:
		e.set16(C.R8_OFF_request_addr, uint16(code[1])|uint16(code[2])<<8)
		e.load8(op & 7)
		e.bytes(0x88, 0x47, C.R8_OFF_request_data)
		e.set8(C.R8_OFF_request, 4)
	case (op == 0x1D && len(code) == 2) || (op == 0x1F && len(code) == 3) || (op == 0x28 && len(code) == 2) || (op == 0x2F && len(code) == 3):
		source, target := code[1]>>4, code[1]&15
		load := op == 0x1D || op == 0x1F
		address, value := target, source
		if load {
			address, value = source, target
		}
		if address < 8 || address > 12 || value >= 8 {
			e.trap(0)
			return e, nil
		}
		e.load16(address)
		if len(code) == 3 {
			e.bytes(0x66, 0x05)
			e.word(uint16(code[2]))
		}
		if load {
			e.readAX()
			e.set8(C.R8_OFF_arg0, value)
			e.next(C.R8_WAIT_DATA)
			return e, nil
		}
		e.bytes(0x66, 0x89, 0x47, C.R8_OFF_request_addr)
		e.load8(value)
		e.bytes(0x88, 0x47, C.R8_OFF_request_data)
		e.set8(C.R8_OFF_request, 4)
	case op == 0x29 && len(code) == 3:
		if code[1] < 8 || code[1] > 12 {
			e.trap(0)
			return e, nil
		}
		e.load16(code[1])
		e.bytes(0x66, 0x89, 0x47, C.R8_OFF_request_addr)
		e.set8(C.R8_OFF_request_data, code[2])
		e.set8(C.R8_OFF_request, 4)
	case ((op == 0x3D || op == 0x4D) && len(code) == 2) || ((op == 0x3E || op == 0x4E) && len(code) == 3):
		if len(code) == 2 {
			if code[1] < 8 || code[1] > 12 {
				e.trap(0)
				return e, nil
			}
			e.load16(code[1])
		} else {
			e.bytes(0xB8, code[1], code[2], 0, 0)
		}
		e.readAX()
		e.bytes(0x66, 0x89, 0x47, C.R8_OFF_arg16)
		state := uint16(C.R8_WAIT_INC)
		if op&0xF0 == 0x40 {
			state = C.R8_WAIT_DEC
		}
		e.next(state)
		return e, nil
	case op >= 0xD0 && op <= 0xDC && len(code) == 1:
		reg := op & 15
		if reg == 12 {
			e.bytes(0x8A, 0x47, C.R8_OFF_flags)
		} else {
			e.loadReg(reg)
		}
		if reg >= 8 && reg < 12 {
			e.bytes(0x88, 0x67, C.R8_OFF_arg0)
		}
		e.pushAL()
		if reg >= 8 && reg < 12 {
			e.next(C.R8_PUSH_DATA)
			return e, nil
		}
	case op >= 0xE0 && op <= 0xEC && len(code) == 1:
		reg := op & 15
		e.pop()
		if reg == 12 {
			e.next(C.R8_WAIT_PS)
		} else {
			e.set8(C.R8_OFF_arg0, reg)
			if reg >= 8 {
				e.next(C.R8_WAIT_STACK_HI)
			} else {
				e.set8(C.R8_OFF_op_hi, 0)
				e.next(C.R8_WAIT_DATA)
			}
		}
		return e, nil
	case (op == 7 || op == 8) && len(code) == 1:
		state := uint16(C.R8_WAIT_RET_LO)
		if op == 8 {
			e.load16(12)
			e.bytes(0x66, 0xFF, 0xC0)
			e.store16(12)
			e.flags(flagNZ, 0, true)
			state = C.R8_WAIT_FLAGS
		}
		e.pop()
		e.next(state)
		return e, nil
	case op == 0xF9 && len(code) == 2:
		e.trap(code[1])
		return e, nil
	case op == 2 && len(code) == 1:
		e.bytes(0xF6, 0x47, C.R8_OFF_flags, 0x40)
		skip := e.branch(0x84)
		e.trap(1)
		e.patch(skip)
	default:
		return nil, errInstruction
	}
	e.finish()
	return e, nil
}
