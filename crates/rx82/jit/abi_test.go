// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"reflect"
	"syscall"
	"testing"
)

func TestCompilerPanicIsContainedAtABIAndOutputsAreCleared(t *testing.T) {
	handle, err := createCompiler(abiVersion, 4096)
	if err != nil {
		t.Fatal(err)
	}
	original := mapMemory
	t.Cleanup(func() {
		mapMemory = original
		if err := destroyCompiler(handle); err != nil {
			t.Error(err)
		}
	})
	mapMemory = func(int, int64, int, int, int) ([]byte, error) { panic("injected compiler failure") }
	status, entry, allocated, message := probeCompile(handle, []byte{1})
	if status != 5 || entry != 0 || allocated != 0 || message == "" {
		t.Fatalf("panic boundary: %d %x %d %q", status, entry, allocated, message)
	}
	mapMemory = func(int, int64, int, int, int) ([]byte, error) { return nil, syscall.ENOMEM }
	status, entry, allocated, message = probeCompile(handle, []byte{1})
	if status != 5 || entry != 0 || allocated != 0 || message == "" {
		t.Fatalf("allocation boundary: %d %x %d %q", status, entry, allocated, message)
	}
	mapMemory = original
	status, entry, allocated, message = probeCompile(handle, []byte{1})
	if status != 0 || entry == 0 || allocated != 4096 || message != "" {
		t.Fatalf("recovery: %d %x %d %q", status, entry, allocated, message)
	}
}

func TestCContextLayout(t *testing.T) {
	want := []uintptr{52, 2, 0, 8, 10, 12, 13, 14, 15, 16, 20, 22, 24, 25, 28, 36, 37, 38, 39, 40, 41, 44, 45, 46, 48, 49}
	if got := contextLayout(); !reflect.DeepEqual(got, want) {
		t.Fatalf("C context layout: %v, want %v", got, want)
	}
}

func TestABIErrorsAreBoundedAndOutputsCleared(t *testing.T) {
	for _, capacity := range []int{0, 1, 4, 256} {
		status, handle, message, guard := probeCreate(abiVersion+1, capacity)
		if status != 1 || handle != 0 || guard != 0x5A {
			t.Fatalf("capacity %d: status %d handle %d guard %x", capacity, status, handle, guard)
		}
		if capacity > 0 && (len(message) >= capacity || (capacity > 1 && len(message) == 0)) {
			t.Fatalf("capacity %d: message %q", capacity, message)
		}
	}
}
