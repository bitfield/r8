// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
package main

import (
	"bytes"
	"testing"
)

func TestRegionKeepsWordRegistersResidentAcrossLoop(t *testing.T) {
	code, _, err := emitRegion(0x100, []byte{0x19, 0xFF, 0xFF, 0x49, 0xF2, 0xFD, 0})
	if err != nil {
		t.Fatal(err)
	}
	// A resident CD pair is published once at the common native exit.
	stores := []byte{0x88, 0x67, 2, 0x88, 0x47, 3}
	if count := bytes.Count(code, stores); count != 1 {
		t.Fatalf("CD publications = %d, want one common exit spill", count)
	}
}

func TestRegionDefersOverwrittenWordNZFlags(t *testing.T) {
	code, _, err := emitRegion(0x100, []byte{0x19, 1, 0, 0x39, 0x39, 0})
	if err != nil {
		t.Fatal(err)
	}
	// The final result supplies NZ at every exit after any retired producer.
	if count := bytes.Count(code, []byte{0x88, 0x57, 12}); count != 1 {
		t.Fatalf("NZ publications = %d, want one shared cold publication", count)
	}
}

func TestRegionPreservesAllCalleeSavedRegistersAtBudgetExits(t *testing.T) {
	code, _, err := emitRegion(0x100, []byte{0x18, 1, 0, 0x19, 2, 0, 0x1A, 3, 0, 0x1B, 4, 0, 0x1C, 0, 0x10, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0})
	if err != nil {
		t.Fatal(err)
	}
	memory, err := publish(code)
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := release(memory); err != nil {
			t.Error(err)
		}
	}()
	for budget := uint32(0); budget <= 70; budget++ {
		status, cycles, regs, sp := probeRegion(memory.entry(), budget)
		if status != 0 || cycles > budget {
			t.Fatalf("budget %d: ABI %d cycles %d", budget, status, cycles)
		}
		if budget >= 64 && (cycles != 64 || regs != [8]byte{0, 2, 0, 3, 0, 4, 0, 5} || sp != 0x1001) {
			t.Fatalf("budget %d: cycles %d registers %x SP %x", budget, cycles, regs, sp)
		}
	}
}

func TestCountLoopUsesFusedNativeAndExactRemainderPaths(t *testing.T) {
	code, _, err := emitRegion(0x100, []byte{0x49, 0xF2, 0xFD, 0})
	if err != nil {
		t.Fatal(err)
	}
	if count := bytes.Count(code, []byte{0x66, 0xFF, 0xC8}); count != 2 {
		t.Fatalf("DEC paths = %d, want fused loop and exact remainder", count)
	}
}
