    ld a, 0x00
    cmp a, 0x00
    beq TEST_PASS
    jmp TEST_FAIL
    ld a, 0x01
    halt

    include lib/test.asm
