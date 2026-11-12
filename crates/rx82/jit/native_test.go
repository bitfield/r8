// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import "testing"

func TestNativeNOPPublishesStateAndPreservesRegisters(t *testing.T) {
	handle, err := createCompiler(abiVersion, 4096)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := destroyCompiler(handle); err != nil {
			t.Error(err)
		}
	})
	compiled, err := compileInstruction(handle, []byte{0x01})
	if err != nil {
		t.Fatal(err)
	}
	if compiled.entry == 0 || compiled.size != 4096 {
		t.Fatalf("block: %+v", compiled)
	}
	status, state, alignment := probeNative(compiled.entry, false)
	if status != 0 || state != 2 || alignment != 0 {
		t.Fatalf("entry: status %d state %d arg %d", status, state, alignment)
	}
	if _, err := compileInstruction(handle, []byte{0x01}); err != errBudget {
		t.Fatalf("budget: %v", err)
	}
	if err := invalidateCompiler(handle); err != nil {
		t.Fatal(err)
	}
	if _, err := compileInstruction(handle, []byte{0x01}); err != nil {
		t.Fatalf("reclaimed budget: %v", err)
	}
}

func TestNativeEntryStackAlignment(t *testing.T) {
	// mov rax,rsp; and eax,15; mov [rdi+arg0],al; ret
	code := []byte{0x48, 0x89, 0xE0, 0x83, 0xE0, 0x0F, 0x88, 0x47, byte(contextLayout()[12]), 0xC3}
	memory, err := publish(code)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := release(memory); err != nil {
			t.Error(err)
		}
	})
	status, _, alignment := probeNative(memory.entry(), true)
	if status != 0 || alignment != 8 {
		t.Fatalf("status %d, entry RSP mod 16 = %d", status, alignment)
	}
}

func TestNativeByteAdditionExhaustive(t *testing.T)    { checkByteALU(t, 0x50) }
func TestNativeByteSubtractionExhaustive(t *testing.T) { checkByteALU(t, 0x60) }
func TestNativeByteComparisonExhaustive(t *testing.T)  { checkByteALU(t, 0x70) }
func TestNativeByteLogicExhaustive(t *testing.T) {
	for _, op := range []byte{0x80, 0x90, 0xA0, 0xB0} {
		checkByteALU(t, op)
	}
}

func checkByteALU(t *testing.T, opcode byte) {
	t.Helper()
	for reg := range byte(8) {
		handle, err := createCompiler(abiVersion, 256*4096)
		if err != nil {
			t.Fatal(err)
		}
		for immediate := range 256 {
			compiled, err := compileInstruction(handle, []byte{opcode + reg, byte(immediate)})
			if err != nil {
				t.Fatal(err)
			}
			for value := range 256 {
				for carry := range 2 {
					total := value + immediate + carry
					result := byte(total)
					wantFlags := byte(4)
					switch opcode {
					case 0x50:
						if total > 255 {
							wantFlags |= 1
						}
						if (byte(value)^byte(immediate))&128 == 0 && (byte(value)^result)&128 != 0 {
							wantFlags |= 64
						}
					case 0x60:
						total = value - immediate - (1 - carry)
						result = byte(total)
						if total >= 0 {
							wantFlags |= 1
						}
						if (byte(value)^byte(immediate))&128 != 0 && (byte(value)^result)&128 != 0 {
							wantFlags |= 64
						}
					case 0x70:
						result = byte(value - immediate)
						wantFlags |= 64
						if value >= immediate {
							wantFlags |= 1
						}
					case 0x80, 0x90:
						result = byte(value & immediate)
						wantFlags |= 64 | byte(carry)
					case 0xA0:
						result = byte(value | immediate)
						wantFlags |= 64 | byte(carry)
					case 0xB0:
						result = byte(value ^ immediate)
						wantFlags |= 64 | byte(carry)
					}
					if result == 0 {
						wantFlags |= 2
					}
					if result&128 != 0 {
						wantFlags |= 128
					}
					expected := result
					if opcode == 0x70 || opcode == 0x90 {
						expected = byte(value)
					}
					got, flags, state, status := probeALU(compiled.entry, reg, byte(value), byte(0xC6|carry))
					if got != expected || flags != wantFlags || state != 2 || status != 0 {
						t.Fatalf("opcode %x r%d lhs=%x rhs=%x carry=%d: got %x flags %x state %d status %d, want %x flags %x", opcode, reg, value, immediate, carry, got, flags, state, status, expected, wantFlags)
					}
				}
			}
		}
		if err := destroyCompiler(handle); err != nil {
			t.Fatal(err)
		}
	}
}
