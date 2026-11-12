#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Zayn Otley
# https://ZaynOtley.com
# https://IntuitionEngine.io
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Compare pinned external R8 implementations with native RX82 execution."""

import argparse
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
REVISIONS = {"gr8": "0ed125d2acee4dcdb1c99747968e0136f074b5c3",
             "go_r8": "05d069e0eb3d75919309c363e19236c2e6966e41",
             "typescript": "234b7947cbae9e40a44a691784857b68d94b7374"}


def run(command, **kwargs):
    result = subprocess.run(command, capture_output=True, text=True, timeout=120, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{command}: {result.stderr}")
    return result.stdout


def cases(kind):
    result = []
    for value in range(256):
        tails = [[0], [0x30, 0], [1, 0x30, 0x30, 0]]
        if kind != "typescript":
            tails += [[0x40, 0], [0x40, 0x30, 0]]
        for tail in tails:
            start = (0x100, 0xFFFE, 0xFFFF)[len(result) % 3]
            result.append((start, 0xC7 if value & 1 else 0, bytes([0x10, value] + tail)))
    if kind == "go_r8":
        for value in range(1, 17):
            result.append((0x100, 0, bytes([0x10, value, 0x40, 0xF2, 0xFD, 0])))
        for value in range(256):
            result.append((0x100, 0, bytes([0x10, value, 0xF0, 1, 0x30, 0])))
            result.append((0x100, 0, bytes([0x10, value, 0xF1, 1, 0x30, 0])))
            result.append((0x100, 0, bytes([0x10, value, 0xF7, 6, 1, 0x30, 0])))
    return result


GO_DRIVER = r'''// SPDX-License-Identifier: AGPL-3.0-or-later
package main
import("bufio";"encoding/hex";"fmt";"os";"strconv";"strings"; ref "MODULE")
func main(){
 scanner:=bufio.NewScanner(os.Stdin)
 for scanner.Scan(){
  fields:=strings.Fields(scanner.Text()); start,_:=strconv.Atoi(fields[0]); flags,_:=strconv.Atoi(fields[1]); code,err:=hex.DecodeString(fields[2]);if err!=nil{panic(err)}
  SETUP
  for i,value:=range code{MEMORY[uint16(start+i)]=value}
  EXECUTE
  OUTPUT
 }
 if err:=scanner.Err();err!=nil{panic(err)}
}
'''


def external_go(kind, path, data):
    module = "github.com/bitfield/gr8" if kind == "gr8" else "github.com/jgrecu/go-r8"
    with tempfile.TemporaryDirectory(prefix="r8-jit-oracle-") as directory:
        directory = Path(directory)
        (directory / "go.mod").write_text(f'module r8-jit-oracle\n\ngo 1.27.0\nrequire {module} v0.0.0\nreplace {module} => {json.dumps(str(path))}\n')
        if kind == "gr8":
            substitutions = {"SETUP": "cpu:=ref.New();cpu.PC=uint16(start);_ = flags", "MEMORY": "cpu.Mem",
                             "EXECUTE": "halted:=false;for range 10000{if !cpu.Step(){halted=true;break}};if !halted{panic(\"step limit\")}",
                             "OUTPUT": 'fmt.Printf("[%d,%d]\\n",cpu.A,cpu.PC)'}
        else:
            substitutions = {"SETUP": "cpu:=ref.NewCPU();cpu.PC=uint16(start);cpu.PS=ref.Flags(flags)", "MEMORY": "cpu.Memory",
                             "EXECUTE": "for steps:=0;steps<10000&&!cpu.Halted;steps++{if err:=cpu.Step();err!=nil{panic(err)}};if !cpu.Halted{panic(\"step limit\")}",
                             "OUTPUT": 'fmt.Printf("[%d,%d,%d]\\n",cpu.R[ref.A],cpu.PC,cpu.PS)'}
        driver = GO_DRIVER.replace("MODULE", module)
        for name, value in substitutions.items():
            driver = driver.replace(name, value)
        (directory / "main.go").write_text(driver)
        return run(["go", "run", "."], cwd=directory, input=data)


def external_typescript(path, data):
    with tempfile.TemporaryDirectory(prefix="r8-jit-oracle-") as directory:
        driver = Path(directory) / "probe.ts"
        driver.write_text('''// SPDX-License-Identifier: AGPL-3.0-or-later
import {readFileSync} from 'node:fs';
import {CreateCPU} from MODULE;
for(const line of readFileSync(0,'utf8').trim().split('\\n')){
 const [start,flags,hex]=line.split(' ');const cpu=CreateCPU();cpu.PC=Number(start);
 const bytes=Buffer.from(hex,'hex');for(let i=0;i<bytes.length;i++)cpu.memory.writeByte((Number(start)+i)&65535,bytes[i]);
 let halted=false;for(let i=0;i<10000;i++){if(!cpu.step()){halted=true;break;}}if(!halted)throw Error('step limit');
 console.log(JSON.stringify([cpu.A,cpu.PC]));
}
'''.replace("MODULE", json.dumps(str(path / "src/cpu/index.ts"))))
        return run(["node", "--import", str(path / "node_modules/tsx/dist/loader.mjs"), str(driver)], cwd=path, input=data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for kind in REVISIONS:
        parser.add_argument("--" + kind.replace("_", "-"), type=Path)
    arguments = parser.parse_args()
    run(["cargo", "build", "-p", "rx82", "--release", "--features", "jit", "--example", "jit_reference"], cwd=ROOT)
    executable = ROOT / "target/release/examples/jit_reference"
    checked = 0
    for kind, revision in REVISIONS.items():
        path = getattr(arguments, kind)
        if path is None:
            continue
        path = path.resolve()
        if run(["git", "rev-parse", "HEAD"], cwd=path).strip() != revision:
            raise RuntimeError(f"{kind}: reference revision differs from inventory")
        corpus = cases(kind)
        data = "".join(f"{start} {flags} {code.hex()}\n" for start, flags, code in corpus)
        actual = [json.loads(line) for line in run([str(executable)], input=data).splitlines()]
        oracle = external_typescript(path, data) if kind == "typescript" else external_go(kind, path, data)
        expected = [json.loads(line) for line in oracle.splitlines()]
        if len(actual) != len(corpus) or len(expected) != len(corpus):
            raise RuntimeError(f"{kind}: incomplete results")
        for index, (native, reference) in enumerate(zip(actual, expected)):
            if native[:len(reference)] != reference or native[3] == 0:
                raise RuntimeError(f"{kind} case {index} {corpus[index]}: native {native}, reference {reference}")
        print(f"{kind} {revision}: {len(corpus)} architectural cases passed")
        checked += 1
    if not checked:
        parser.error("provide at least one pinned reference checkout")


if __name__ == "__main__":
    main()
