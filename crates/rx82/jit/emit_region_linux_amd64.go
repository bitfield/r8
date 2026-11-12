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
	"bytes"
	"encoding/binary"
)

// regionLength admits pure instructions and guarded byte RAM transfers.
// Other instructions resume through the exact cycle backend.
func regionLength(op byte) int {
	switch {
	case op == 0 || op == 1 || op >= 3 && op <= 6:
		return 1
	case op >= 0x10 && op <= 0x1C:
		if op&15 >= 8 {
			return 3
		}
		return 2
	case op == 0x1D || op == 0x28:
		return 2
	case op == 0x1F || op >= 0x20 && op <= 0x27 || op == 0x29 || op == 0x2F:
		return 3
	case op >= 0x30 && op <= 0x3C || op >= 0x40 && op <= 0x4C:
		return 1
	case op >= 0x50 && op <= 0xBC && op&15 <= 12:
		if op&15 >= 8 {
			return 3
		}
		return 2
	case op >= 0xF0 && op <= 0xF6:
		return 2
	case op == 0xF7:
		return 3
	}
	return 0
}

func (e *emitter) dword(value uint32) {
	*e = binary.LittleEndian.AppendUint32(*e, value)
}

// regionMemory validates the same address and value register forms as the
// cycle backend. Invalid encodings retain its trap path.
func regionMemory(code []byte) (memory, load bool) {
	op := code[0]
	if op >= 0x20 && op <= 0x27 {
		return true, false
	}
	if op == 0x29 {
		return code[1] >= 8 && code[1] <= 12, false
	}
	if op != 0x1D && op != 0x1F && op != 0x28 && op != 0x2F {
		return false, false
	}
	address, value := code[1]&15, code[1]>>4
	load = op == 0x1D || op == 0x1F
	if load {
		address, value = value, address
	}
	return address >= 8 && address <= 12 && value < 8, load
}

func (e *emitter) ramAddress(code []byte, cache *wordCache) {
	switch op := code[0]; {
	case op >= 0x20 && op <= 0x27:
		e.bytes(0xB8, code[1], code[2], 0, 0)
	default:
		address := code[1] & 15
		if op == 0x1D || op == 0x1F {
			address = code[1] >> 4
		}
		cache.load(e, address)
		if op == 0x1F || op == 0x2F {
			e.bytes(0x66, 0x05)
			e.word(uint16(code[2]))
		}
	}
}

// A store becomes visible to RAM on the following fetch cycle, matching RX82.
// A budget exit before that fetch leaves the write pending on the bus.
func (e *emitter) commitRAMWrite() {
	e.bytes(0x80, 0x7F, C.R8_REGION_WRITE, 0)
	skip := e.branch(0x84)
	e.bytes(0x0F, 0xB7, 0x47, C.R8_REGION_ADDR)
	e.bytes(0x0F, 0xB7, 0x4F, C.R8_REGION_RAM_START, 0x29, 0xC8)
	e.bytes(0x0F, 0xB6, 0x57, C.R8_OFF_bus_data)
	e.bytes(0x41, 0x88, 0x54, 0x05, 0) // mov [r13+rax],dl
	e.patch(skip)
}

func (e *emitter) ramTransfer(code []byte, load bool) {
	// ESI contains the guarded physical offset and is untouched by commit.
	e.bytes(0x89, 0xF0, 0x0F, 0xB7, 0x4F, C.R8_REGION_RAM_START, 0x01, 0xC8)
	e.bytes(0x66, 0x89, 0x47, C.R8_REGION_ADDR)
	e.set8(C.R8_REGION_MEM, 1)
	if load {
		e.bytes(0x41, 0x0F, 0xB6, 0x44, 0x35, 0) // movzx eax,byte [r13+rsi]
		e.store8(code[1] & 15)
		e.bytes(0x88, 0x47, C.R8_OFF_op_lo, 0x88, 0x47, C.R8_OFF_bus_data)
		e.flags(flagNZ, 0, false)
	} else {
		switch op := code[0]; {
		case op >= 0x20 && op <= 0x27:
			e.load8(op & 7)
		case op == 0x29:
			e.bytes(0xB0, code[2])
		default:
			e.load8(code[1] >> 4)
		}
		e.bytes(0x88, 0x47, C.R8_OFF_bus_data)
		e.set8(C.R8_REGION_WRITE, 1)
	}
}

func deferredWordNZ(code []byte, cache *wordCache) bool {
	op, reg := code[0]&0xF0, code[0]&15
	return cache.slot(reg) >= 0 && (op == 0x10 || op == 0x30 || op == 0x40 || op == 0x80 || op == 0xA0 || op == 0xB0)
}

func (e *emitter) publishWordNZ(cache *wordCache, reg int) {
	if reg >= 0 {
		cache.load(e, byte(reg))
		e.flags(flagNZ, 0, true)
	}
}

func regionTargets(pc uint16, code []byte) map[uint16]bool {
	targets := make(map[uint16]bool)
	for offset := 0; offset < len(code); {
		op := code[offset]
		length := regionLength(op)
		if length == 0 || offset+length > len(code) {
			break
		}
		if op >= 0xF0 && op <= 0xF7 {
			target := pc + uint16(offset+length) + uint16(int16(int8(code[offset+1])))
			if op == 0xF7 {
				target = binary.LittleEndian.Uint16(code[offset+1:])
			}
			targets[target] = true
		}
		offset += length
		if op == 0 {
			break
		}
	}
	return targets
}

// emitRegion shares pure instruction lowering with the cycle backend. Direct
// RAM transfers use guarded slices. R12 holds the remaining cycle budget, R11
// counts retired cycles and R13 holds the borrowed RAM pointer.
func emitRegion(pc uint16, code []byte) ([]byte, int, error) {
	var e emitter
	e.bytes(0x41, 0x54, 0x41, 0x55)
	e.bytes(0x44, 0x8B, 0x67, C.R8_REGION_BUDGET, 0x45, 0x31, 0xDB)
	e.bytes(0x4C, 0x8B, 0x6F, C.R8_REGION_RAM)
	cache := planWordCache(code)
	cache.save(&e)
	starts := make(map[uint16]int)
	type link struct {
		at     int
		target uint16
	}
	var links []link
	type regionExit struct {
		at int
		nz int
	}
	var exits []regionExit
	pendingNZ := -1
	targets := regionTargets(pc, code)
	consumed := 0
	for consumed < len(code) {
		op := code[consumed]
		length := regionLength(op)
		if length == 0 || consumed+length > len(code) {
			break
		}
		instruction := code[consumed : consumed+length]
		memory, load := regionMemory(instruction)
		var body []byte
		var suffix emitter
		suffix.finish()
		if !memory {
			var err error
			if op >= 0x10 && op <= 0xBC && op&15 >= 8 && op&15 <= 12 {
				body, err = emitWordCached(instruction, cache, deferredWordNZ(instruction, cache))
			} else {
				body, err = emitExecute(instruction)
			}
			if err != nil || !bytes.HasSuffix(body, suffix) {
				break
			}
		}
		address := pc + uint16(consumed)
		if targets[address] {
			e.publishWordNZ(cache, pendingNZ)
			pendingNZ = -1
		}
		starts[address] = len(e)
		if countLoop(code[consumed:], cache) {
			links = append(links, link{e.fusedCountLoop(op, address, cache), address + 3})
		}
		cost := byte(2*length + 2)
		if load {
			cost += 2
		}
		e.bytes(0x41, 0x83, 0xFC, cost)
		exits = append(exits, regionExit{e.branch(0x82), pendingNZ})
		if memory {
			e.ramAddress(instruction, cache)
			// Both bounds and backing length are represented by ram_length.
			e.bytes(0x0F, 0xB7, 0x4F, C.R8_REGION_RAM_START, 0x29, 0xC8)
			e.bytes(0x3B, 0x47, C.R8_REGION_RAM_LENGTH)
			exits = append(exits, regionExit{e.branch(0x83), pendingNZ})
			e.bytes(0x89, 0xC6) // mov esi,eax
		}
		e.commitRAMWrite()
		e.bytes(0x41, 0x83, 0xEC, cost, 0x41, 0x83, 0xC3, cost)
		next := address + uint16(length)
		e.set16(C.R8_OFF_pc, next)
		e.set8(C.R8_OFF_ins, op)
		if length >= 2 {
			e.set8(C.R8_OFF_op_lo, instruction[1])
			hi := byte(0)
			if length == 3 {
				hi = instruction[2]
			}
			e.set8(C.R8_OFF_op_hi, hi)
		}
		e.set8(C.R8_OFF_bus_data, instruction[length-1])
		e.set16(C.R8_REGION_ADDR, next-1)
		mem := byte(0)
		if length > 1 {
			mem = 1
		}
		e.set8(C.R8_REGION_MEM, mem)
		e.set8(C.R8_REGION_WRITE, 0)
		if op >= 0xF0 && op <= 0xF7 {
			e.publishWordNZ(cache, pendingNZ)
			pendingNZ = -1
		}
		if memory {
			e.ramTransfer(instruction, load)
		} else {
			e.bytes(body[:len(body)-len(suffix)]...)
		}
		if deferredWordNZ(instruction, cache) && !memory {
			pendingNZ = int(op & 15)
		} else if load || op >= 0x10 && op <= 0xBC && !memory {
			pendingNZ = -1
		}
		consumed += length
		if op == 0 {
			break
		}
		if memory && !load {
			// Stop before fetching code changed by a deferred store. Rust will
			// commit it on the next fetch and validate regions before re-entry.
			e.bytes(0x0F, 0xB7, 0x47, C.R8_REGION_ADDR, 0x2D)
			e.dword(uint32(pc))
			e.bytes(0x3D)
			e.dword(uint32(len(code)))
			exits = append(exits, regionExit{e.branch(0x82), pendingNZ})
		}
		if op >= 0xF0 && op <= 0xF7 {
			target := uint16(0)
			if op == 0xF7 {
				target = binary.LittleEndian.Uint16(instruction[1:])
			} else {
				target = next + uint16(int16(int8(instruction[1])))
			}
			if op == 0xF0 || op == 0xF7 {
				e.bytes(0xE9)
				e.dword(0)
				links = append(links, link{len(e) - 4, target})
			} else if target != next {
				e.bytes(0x66, 0x81, 0x7F, C.R8_OFF_pc)
				e.word(target)
				links = append(links, link{e.branch(0x84), target})
			}
		}
	}
	if consumed == 0 {
		return nil, 0, errInstruction
	}
	// Each guard leaves through a cold stub for the last deferred NZ producer.
	// Branch destinations enter with canonical flags, including forward edges.
	e.bytes(0xE9)
	e.dword(0)
	exits = append(exits, regionExit{len(e) - 4, pendingNZ})
	stubs := make(map[int]int)
	var common []int
	emitStub := func(reg int) {
		if _, exists := stubs[reg]; exists {
			return
		}
		stubs[reg] = len(e)
		e.publishWordNZ(cache, reg)
		e.bytes(0xE9)
		e.dword(0)
		common = append(common, len(e)-4)
	}
	emitStub(-1)
	for _, exit := range exits {
		emitStub(exit.nz)
	}
	for _, jump := range common {
		e.patch(jump)
	}
	for _, link := range links {
		target, ok := starts[link.target]
		if !ok {
			target = stubs[-1]
		}
		e.patchTo(link.at, target)
	}
	for _, exit := range exits {
		target := stubs[exit.nz]
		e.patchTo(exit.at, target)
	}
	e.bytes(0x44, 0x89, 0x5F, C.R8_REGION_CYCLES)
	e.set16(C.R8_OFF_state, C.R8_FETCH_OPCODE)
	cache.restore(&e)
	e.bytes(0x41, 0x5D, 0x41, 0x5C, 0xC3)
	return e, consumed, nil
}
