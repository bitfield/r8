// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"fmt"
	"syscall"
	"unsafe"
)

type executable []byte

var protectMemory = syscall.Mprotect
var unmapMemory = syscall.Munmap
var mapMemory = syscall.Mmap

func allocationSize(length int) uint64 {
	page := uint64(syscall.Getpagesize())
	return (uint64(length) + page - 1) / page * page
}

func publish(code []byte) (executable, error) {
	if len(code) == 0 {
		return nil, errInstruction
	}
	memory, err := mapMemory(-1, 0, int(allocationSize(len(code))), syscall.PROT_READ|syscall.PROT_WRITE, syscall.MAP_PRIVATE|syscall.MAP_ANON)
	if err != nil {
		return nil, fmt.Errorf("allocate native code: %w", err)
	}
	copy(memory, code)
	if err := protectMemory(memory, syscall.PROT_READ|syscall.PROT_EXEC); err != nil {
		if releaseErr := unmapMemory(memory); releaseErr != nil {
			return memory, errorsJoinPublication(err, releaseErr)
		}
		return nil, errorsJoinPublication(err, nil)
	}
	return memory, nil
}

func errorsJoinPublication(protect, release error) error {
	if release != nil {
		return fmt.Errorf("publish native code: %v; release: %w", protect, release)
	}
	return fmt.Errorf("publish native code: %w", protect)
}

func (memory executable) entry() uintptr { return uintptr(unsafe.Pointer(&memory[0])) }

func release(memory executable) error { return unmapMemory(memory) }
