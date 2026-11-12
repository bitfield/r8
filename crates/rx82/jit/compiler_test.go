// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"errors"
	"testing"
)

func TestCompilerLifecycle(t *testing.T) {
	if _, err := createCompiler(abiVersion+1, 4096); !errors.Is(err, errVersion) {
		t.Fatalf("version mismatch: %v", err)
	}
	if _, err := createCompiler(abiVersion, 0); !errors.Is(err, errBudget) {
		t.Fatalf("zero budget: %v", err)
	}
	var previous uint64
	for range 100 {
		handle, err := createCompiler(abiVersion, 4096)
		if err != nil || handle == 0 || handle <= previous {
			t.Fatalf("create: handle %d after %d, error %v", handle, previous, err)
		}
		if err := invalidateCompiler(handle); err != nil {
			t.Fatal(err)
		}
		if err := destroyCompiler(handle); err != nil {
			t.Fatal(err)
		}
		if err := destroyCompiler(handle); !errors.Is(err, errHandle) {
			t.Fatalf("stale destroy: %v", err)
		}
		if err := invalidateCompiler(handle); !errors.Is(err, errHandle) {
			t.Fatalf("stale invalidate: %v", err)
		}
		previous = handle
	}
	if err := destroyCompiler(0); !errors.Is(err, errHandle) {
		t.Fatalf("zero handle: %v", err)
	}
}

func TestCompilationFailureDoesNotDestroyCompiler(t *testing.T) {
	handle, err := createCompiler(abiVersion, 4096)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := destroyCompiler(handle); err != nil {
			t.Error(err)
		}
	})
	for _, code := range [][]byte{nil, {0xFF}, {0x01, 0x00, 0x00, 0x00}} {
		if _, err := compileInstruction(handle, code); !errors.Is(err, errInstruction) {
			t.Fatalf("compile %x: %v", code, err)
		}
	}
	if _, err := compileInstruction(0, []byte{0x01}); !errors.Is(err, errHandle) {
		t.Fatalf("bad handle: %v", err)
	}
	if err := invalidateCompiler(handle); err != nil {
		t.Fatal(err)
	}
}
