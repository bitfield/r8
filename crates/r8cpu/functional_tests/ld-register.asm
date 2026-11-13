    ld b, 0xFF
    ld a, b
    cmp a, 0xFF
    bne TEST_FAIL

    ld ab, 0xABCD
    ld cd, ab
    cmp cd, 0xABCD
    bne TEST_FAIL

    include lib/test.asm
