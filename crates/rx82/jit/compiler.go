// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"errors"
	"sync"
)

var (
	errVersion     = errors.New("unsupported JIT ABI version")
	errBudget      = errors.New("invalid executable memory budget")
	errHandle      = errors.New("invalid compiler handle")
	errInstruction = errors.New("instruction is not supported")
	compilers      = struct {
		sync.Mutex
		next      uint64
		instances map[uint64]*compiler
	}{instances: make(map[uint64]*compiler)}
)

type compiler struct {
	budget      uint64
	used        uint64
	allocations []executable
}

type block struct {
	entry uintptr
	size  uint64
}

func createCompiler(version uint32, budget uint64) (uint64, error) {
	if version != abiVersion {
		return 0, errVersion
	}
	if budget == 0 {
		return 0, errBudget
	}
	compilers.Lock()
	defer compilers.Unlock()
	if compilers.next == ^uint64(0) {
		return 0, errHandle
	}
	compilers.next++
	compilers.instances[compilers.next] = &compiler{budget: budget}
	return compilers.next, nil
}

func invalidateCompiler(handle uint64) error {
	compilers.Lock()
	defer compilers.Unlock()
	instance := compilers.instances[handle]
	if instance == nil {
		return errHandle
	}
	return instance.clear()
}

func destroyCompiler(handle uint64) error {
	compilers.Lock()
	defer compilers.Unlock()
	instance := compilers.instances[handle]
	if instance == nil {
		return errHandle
	}
	if err := instance.clear(); err != nil {
		return err
	}
	delete(compilers.instances, handle)
	return nil
}

func compileInstruction(handle uint64, code []byte) (block, error) {
	compilers.Lock()
	defer compilers.Unlock()
	instance := compilers.instances[handle]
	if instance == nil {
		return block{}, errHandle
	}
	native, err := emitInstruction(code)
	if err != nil {
		return block{}, err
	}
	return instance.publish(native)
}

func (instance *compiler) publish(native []byte) (block, error) {
	if instance.budget-instance.used < allocationSize(len(native)) {
		return block{}, errBudget
	}
	memory, err := publish(native)
	// Retain failed RW mappings if the OS also refused their release.
	if memory != nil {
		instance.allocations = append(instance.allocations, memory)
		instance.used += uint64(len(memory))
	}
	if err != nil {
		return block{}, err
	}
	return block{entry: memory.entry(), size: uint64(len(memory))}, nil
}

func (instance *compiler) clear() error {
	var failures error
	remaining := instance.allocations[:0]
	for _, memory := range instance.allocations {
		if err := release(memory); err != nil {
			failures = errors.Join(failures, err)
			remaining = append(remaining, memory)
		} else {
			instance.used -= uint64(len(memory))
		}
	}
	instance.allocations = remaining
	return failures
}

// releaseEntry removes one stopped native entry without discarding other code.
func releaseEntry(handle uint64, entry uintptr) error {
	compilers.Lock()
	defer compilers.Unlock()
	instance := compilers.instances[handle]
	if instance == nil {
		return errHandle
	}
	for i, memory := range instance.allocations {
		if memory.entry() != entry {
			continue
		}
		if err := release(memory); err != nil {
			return err
		}
		instance.used -= uint64(len(memory))
		instance.allocations = append(instance.allocations[:i], instance.allocations[i+1:]...)
		return nil
	}
	return errArgument
}
