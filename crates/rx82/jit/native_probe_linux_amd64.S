/* SPDX-FileCopyrightText: 2026 Zayn Otley
 * https://ZaynOtley.com
 * https://IntuitionEngine.io
 * SPDX-License-Identifier: AGPL-3.0-or-later */
.text
.globl r8_probe_entry
.type r8_probe_entry,@function
r8_probe_entry:
    push %rbx
    push %rbp
    push %r12
    push %r13
    push %r14
    push %r15
    sub $24, %rsp
    mov %rdi, (%rsp)
    mov %rsi, %rdi
    mov $0x1234, %ebx
    mov $0x2345, %ebp
    mov $0x3456, %r12d
    mov $0x4567, %r13d
    mov $0x5678, %r14d
    mov $0x6789, %r15d
    call *(%rsp)
    mov $1, %eax
    cmp $0x1234, %rbx
    jne .Ldone
    cmp $0x2345, %rbp
    jne .Ldone
    cmp $0x3456, %r12
    jne .Ldone
    cmp $0x4567, %r13
    jne .Ldone
    cmp $0x5678, %r14
    jne .Ldone
    cmp $0x6789, %r15
    jne .Ldone
    xor %eax, %eax
.Ldone:
    add $24, %rsp
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbp
    pop %rbx
    ret
.size r8_probe_entry,.-r8_probe_entry
.section .note.GNU-stack,"",@progbits
