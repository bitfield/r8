// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

/*
#include "r8jit.h"
*/
import "C"

import "encoding/binary"

func (e *emitter) branch(condition byte) int {
	e.bytes(0x0F, condition, 0, 0, 0, 0)
	return len(*e) - 4
}
func (e *emitter) patch(branch int) { e.patchTo(branch, len(*e)) }
func (e *emitter) relativeJump() int {
	e.bytes(0xE9)
	e.dword(0)
	return len(*e) - 4
}
func (e *emitter) patchTo(at, target int) {
	binary.LittleEndian.PutUint32((*e)[at:at+4], uint32(int32(target-at-4)))
}
func (e *emitter) next(state uint16)         { e.set16(C.R8_OFF_state, state); e.bytes(0xC3) }
func (e *emitter) copy8(source, target byte) { e.bytes(0x8A, 0x47, source, 0x88, 0x47, target) }
func (e *emitter) load8(reg byte)            { e.bytes(0x0F, 0xB6, 0x47, C.R8_OFF_regs+reg) }
func (e *emitter) store8(reg byte)           { e.bytes(0x88, 0x47, C.R8_OFF_regs+reg) }
func (e *emitter) loadReg(reg byte) {
	if reg < 8 {
		e.load8(reg)
	} else {
		e.load16(reg)
	}
}
func (e *emitter) storeReg(reg byte) {
	if reg < 8 {
		e.store8(reg)
	} else {
		e.store16(reg)
	}
}
func (e *emitter) operand() {
	e.bytes(0x0F, 0xB6, 0x47, C.R8_OFF_op_hi, 0xC1, 0xE0, 8, 0x8A, 0x47, C.R8_OFF_op_lo)
}
func (e *emitter) readAX() {
	e.bytes(0x66, 0x89, 0x47, C.R8_OFF_request_addr)
	e.set8(C.R8_OFF_request, 2)
}
func (e *emitter) pop() {
	e.load16(12)
	e.bytes(0x66, 0xFF, 0xC0)
	e.store16(12)
	e.readAX()
}
func (e *emitter) trap(code byte) {
	if code >= 0x40 {
		code = 0
	}
	if code == 0x20 {
		e.set8(C.R8_OFF_host_output, 1)
	}
	e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_pc, 0x88, 0x47, C.R8_OFF_arg0, 0xC1, 0xE8, 8)
	e.set8(C.R8_OFF_arg1, code)
	e.pushAL()
	e.next(C.R8_PUSH_RET_LO)
}
func (e *emitter) stateCase(state uint16, body func()) {
	e.bytes(0x66, 0x83, 0x7F, C.R8_OFF_state, byte(state))
	skip := e.branch(0x85)
	body()
	e.patch(skip)
}

// Each block resumes one cycle at a time. All payloads are current context input.
func (e *emitter) continuations() {
	for _, transition := range [][2]uint16{
		{C.R8_WAIT_DATA, C.R8_READ_DATA}, {C.R8_WAIT_DEC, C.R8_READ_DEC},
		{C.R8_WAIT_FLAGS, C.R8_READ_FLAGS}, {C.R8_WAIT_INC, C.R8_READ_INC},
		{C.R8_WAIT_PS, C.R8_READ_PS}, {C.R8_WAIT_RET_LO, C.R8_READ_RET_LO},
		{C.R8_WAIT_STACK_HI, C.R8_READ_STACK_HI}, {C.R8_WAIT_VEC_HI, C.R8_READ_VEC_HI},
		{C.R8_WAIT_VEC_LO, C.R8_READ_VEC_LO},
	} {
		e.stateCase(transition[0], func() { e.next(transition[1]) })
	}
	e.stateCase(C.R8_PUSH_DATA, func() { e.bytes(0x8A, 0x47, C.R8_OFF_arg0); e.pushAL(); e.finish() })
	e.stateCase(C.R8_PUSH_RET_LO, func() {
		e.bytes(0x8A, 0x47, C.R8_OFF_arg0)
		e.pushAL()
		e.copy8(C.R8_OFF_arg1, C.R8_OFF_arg0)
		e.next(C.R8_PUSH_FLAGS)
	})
	e.stateCase(C.R8_PUSH_FLAGS, func() { e.bytes(0x8A, 0x47, C.R8_OFF_flags); e.pushAL(); e.next(C.R8_PUSH_TRAP) })
	e.stateCase(C.R8_PUSH_TRAP, func() {
		e.bytes(0x0F, 0xB6, 0x47, C.R8_OFF_arg0)
		e.pushAL()
		e.bytes(0xD1, 0xE0, 0x66, 0x89, 0x47, C.R8_OFF_arg16)
		e.next(C.R8_REQ_VEC_LO)
	})
	e.stateCase(C.R8_REQ_VEC_LO, func() {
		e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_arg16)
		e.readAX()
		e.bytes(0x66, 0xFF, 0xC0, 0x66, 0x89, 0x47, C.R8_OFF_arg16)
		e.next(C.R8_WAIT_VEC_LO)
	})
	e.stateCase(C.R8_READ_VEC_LO, func() {
		e.copy8(C.R8_OFF_bus_data, C.R8_OFF_op_lo)
		e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_arg16)
		e.readAX()
		e.next(C.R8_WAIT_VEC_HI)
	})
	e.stateCase(C.R8_READ_VEC_HI, func() {
		e.copy8(C.R8_OFF_bus_data, C.R8_OFF_op_hi)
		e.operand()
		e.bytes(0x66, 0x89, 0x47, C.R8_OFF_pc)
		e.finish()
	})
	e.stateCase(C.R8_WAIT_CALL, func() {
		e.bytes(0x8A, 0x47, C.R8_OFF_arg0)
		e.pushAL()
		e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_arg16, 0x66, 0x89, 0x47, C.R8_OFF_pc)
		e.finish()
	})
	e.stateCase(C.R8_READ_STACK_HI, func() { e.copy8(C.R8_OFF_bus_data, C.R8_OFF_op_hi); e.pop(); e.next(C.R8_WAIT_DATA) })
	e.stateCase(C.R8_READ_RET_LO, func() { e.copy8(C.R8_OFF_bus_data, C.R8_OFF_op_lo); e.pop(); e.next(C.R8_WAIT_VEC_HI) })
	e.stateCase(C.R8_READ_PS, func() {
		e.copy8(C.R8_OFF_bus_data, C.R8_OFF_flags)
		e.bytes(0x80, 0x67, C.R8_OFF_flags, 0xC7)
		e.finish()
	})
	e.stateCase(C.R8_READ_FLAGS, func() {
		e.copy8(C.R8_OFF_bus_data, C.R8_OFF_flags)
		e.bytes(0x80, 0x67, C.R8_OFF_flags, 0xC7)
		e.pop()
		e.next(C.R8_WAIT_RET_LO)
	})
	for _, state := range []uint16{C.R8_READ_INC, C.R8_READ_DEC} {
		e.stateCase(state, func() {
			e.bytes(0x8A, 0x47, C.R8_OFF_bus_data, 0xFE)
			if state == C.R8_READ_INC {
				e.bytes(0xC0)
			} else {
				e.bytes(0xC8)
			}
			e.bytes(0x88, 0x47, C.R8_OFF_request_data)
			e.flags(flagNZ, 0, false)
			e.bytes(0x0F, 0xB7, 0x47, C.R8_OFF_arg16, 0x66, 0x89, 0x47, C.R8_OFF_request_addr)
			e.set8(C.R8_OFF_request, 4)
			e.finish()
		})
	}
	e.stateCase(C.R8_READ_DATA, func() {
		e.copy8(C.R8_OFF_bus_data, C.R8_OFF_op_lo)
		for reg := byte(0); reg < 13; reg++ {
			e.bytes(0x80, 0x7F, C.R8_OFF_arg0, reg)
			skip := e.branch(0x85)
			if reg < 8 {
				e.bytes(0x0F, 0xB6, 0x47, C.R8_OFF_op_lo)
			} else {
				e.operand()
			}
			e.storeReg(reg)
			e.flags(flagNZ, 0, reg >= 8)
			e.finish()
			e.patch(skip)
		}
		e.next(0xFFFF)
	})
	e.next(0xFFFF)
}

func emitInstruction(code []byte) ([]byte, error) {
	body, err := emitExecute(code)
	if err != nil {
		return nil, err
	}
	var e emitter
	e.bytes(0x66, 0x83, 0x7F, C.R8_OFF_state, C.R8_EXECUTE)
	execute := e.branch(0x84)
	e.continuations()
	e.patch(execute)
	e.bytes(body...)
	return e, nil
}
