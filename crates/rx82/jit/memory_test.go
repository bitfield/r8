// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"errors"
	"os"
	"strconv"
	"strings"
	"syscall"
	"testing"
)

func permissions(t *testing.T, entry uintptr) string {
	t.Helper()
	data, err := os.ReadFile("/proc/self/maps")
	if err != nil {
		t.Fatal(err)
	}
	for line := range strings.SplitSeq(string(data), "\n") {
		fields := strings.Fields(line)
		if len(fields) < 2 {
			continue
		}
		limits := strings.Split(fields[0], "-")
		lo, err := strconv.ParseUint(limits[0], 16, 64)
		if err != nil {
			t.Fatal(err)
		}
		hi, err := strconv.ParseUint(limits[1], 16, 64)
		if err != nil {
			t.Fatal(err)
		}
		if uint64(entry) >= lo && uint64(entry) < hi {
			return fields[1]
		}
	}
	return ""
}

func TestPublishedMemoryIsRXAndReleased(t *testing.T) {
	memory, err := publish([]byte{0xC3})
	if err != nil {
		t.Fatal(err)
	}
	entry := memory.entry()
	if got := permissions(t, entry); got != "r-xp" {
		t.Fatalf("published permissions %q", got)
	}
	if err := release(memory); err != nil {
		t.Fatal(err)
	}
	if got := permissions(t, entry); got != "" {
		t.Fatalf("released mapping remains %q", got)
	}
}

func TestPublicationFailureRetainsOwnershipWhenReleaseFails(t *testing.T) {
	handle, err := createCompiler(abiVersion, 4096)
	if err != nil {
		t.Fatal(err)
	}
	savedProtect, savedUnmap := protectMemory, unmapMemory
	t.Cleanup(func() {
		protectMemory, unmapMemory = savedProtect, savedUnmap
		if err := destroyCompiler(handle); err != nil {
			t.Error(err)
		}
	})
	failure := errors.New("injected protection failure")
	protectMemory = func([]byte, int) error { return failure }
	unmapMemory = func([]byte) error { return syscall.EBUSY }
	result, err := compileInstruction(handle, []byte{1})
	if err == nil || result.entry != 0 {
		t.Fatalf("failed publication returned %+v, %v", result, err)
	}
	instance := compilers.instances[handle]
	if instance.used != 4096 || len(instance.allocations) != 1 {
		t.Fatalf("lost ownership: %+v", instance)
	}
	if got := permissions(t, instance.allocations[0].entry()); got != "rw-p" {
		t.Fatalf("failed mapping permissions %q", got)
	}
	protectMemory, unmapMemory = savedProtect, savedUnmap
	if err := invalidateCompiler(handle); err != nil {
		t.Fatal(err)
	}
	if instance.used != 0 || len(instance.allocations) != 0 {
		t.Fatalf("failed reclamation: %+v", instance)
	}
	if _, err := compileInstruction(handle, []byte{1}); err != nil {
		t.Fatal(err)
	}
}
