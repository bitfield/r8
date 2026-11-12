// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import "testing"

// BenchmarkCompileCorpus measures emission, publication and reclamation across
// the supported opcode families.
func BenchmarkCompileCorpus(b *testing.B) {
	var corpus [][]byte
	for opcode := range 256 {
		for length := 1; length <= 3; length++ {
			code := []byte{byte(opcode), 0, 0}[:length]
			if _, err := emitInstruction(code); err == nil {
				corpus = append(corpus, code)
				break
			}
		}
	}
	if len(corpus) != 211 {
		b.Fatalf("expected 211 supported opcodes, got %d", len(corpus))
	}
	handle, err := createCompiler(abiVersion, 256*4096)
	if err != nil {
		b.Fatal(err)
	}
	b.Cleanup(func() {
		if err := destroyCompiler(handle); err != nil {
			b.Error(err)
		}
	})
	b.ReportAllocs()
	b.ResetTimer()
	for b.Loop() {
		for _, code := range corpus {
			if _, err := compileInstruction(handle, code); err != nil {
				b.Fatal(err)
			}
		}
		if err := invalidateCompiler(handle); err != nil {
			b.Fatal(err)
		}
	}
}
