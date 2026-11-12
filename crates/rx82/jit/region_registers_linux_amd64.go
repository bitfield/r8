// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
package main

import "slices"

import "sort"

// wordCache keeps up to four pairs in callee-saved host registers. A pair is
// excluded if this region touches either byte alias. SP has no byte aliases.
type wordCache struct{ guests []byte }

func planWordCache(code []byte) *wordCache {
	var counts [5]int
	var aliases [5]bool
	byteUse := func(reg byte) {
		if reg < 8 {
			aliases[reg/2] = true
		}
	}
	wordUse := func(reg byte) {
		if reg >= 8 && reg <= 12 {
			counts[reg-8]++
		}
	}
	for offset := 0; offset < len(code); {
		op := code[offset]
		length := regionLength(op)
		if length == 0 || offset+length > len(code) {
			break
		}
		instruction := code[offset : offset+length]
		memory, load := regionMemory(instruction)
		switch {
		case memory:
			switch {
			case op >= 0x20 && op <= 0x27:
				byteUse(op & 7)
			case op == 0x29:
				wordUse(instruction[1])
			case load:
				wordUse(instruction[1] >> 4)
				byteUse(instruction[1] & 15)
			default:
				wordUse(instruction[1] & 15)
				byteUse(instruction[1] >> 4)
			}
		case op >= 0x10 && op <= 0xBC:
			reg := op & 15
			if reg < 8 {
				byteUse(reg)
			} else {
				wordUse(reg)
			}
		}
		offset += length
		if op == 0 {
			break
		}
	}
	cache := &wordCache{}
	for i, count := range counts {
		if count != 0 && !aliases[i] {
			cache.guests = append(cache.guests, byte(i+8))
		}
	}
	sort.Slice(cache.guests, func(i, j int) bool {
		a, b := cache.guests[i], cache.guests[j]
		if counts[a-8] != counts[b-8] {
			return counts[a-8] > counts[b-8]
		}
		return a < b
	})
	if len(cache.guests) > 4 {
		cache.guests = cache.guests[:4]
	}
	return cache
}

func (cache *wordCache) slot(reg byte) int {
	if cache != nil {
		for i, guest := range cache.guests {
			if reg == guest {
				return i
			}
		}
	}
	return -1
}

func (cache *wordCache) save(e *emitter) {
	for i, reg := range cache.guests {
		switch i {
		case 0:
			e.bytes(0x41, 0x56)
		case 1:
			e.bytes(0x41, 0x57)
		case 2:
			e.bytes(0x53)
		case 3:
			e.bytes(0x55)
		}
		e.load16(reg)
		cache.store(e, reg)
	}
}

func (cache *wordCache) restore(e *emitter) {
	for _, reg := range cache.guests {
		cache.load(e, reg)
		e.store16(reg)
	}
	for i := range slices.Backward(cache.guests) {
		switch i {
		case 0:
			e.bytes(0x41, 0x5E)
		case 1:
			e.bytes(0x41, 0x5F)
		case 2:
			e.bytes(0x5B)
		case 3:
			e.bytes(0x5D)
		}
	}
}

func (cache *wordCache) load(e *emitter, reg byte) {
	switch cache.slot(reg) {
	case 0:
		e.bytes(0x44, 0x89, 0xF0)
	case 1:
		e.bytes(0x44, 0x89, 0xF8)
	case 2:
		e.bytes(0x89, 0xD8)
	case 3:
		e.bytes(0x89, 0xE8)
	default:
		e.load16(reg)
	}
}

func (cache *wordCache) store(e *emitter, reg byte) {
	switch cache.slot(reg) {
	case 0:
		e.bytes(0x41, 0x89, 0xC6)
	case 1:
		e.bytes(0x41, 0x89, 0xC7)
	case 2:
		e.bytes(0x89, 0xC3)
	case 3:
		e.bytes(0x89, 0xC5)
	default:
		e.store16(reg)
	}
}
