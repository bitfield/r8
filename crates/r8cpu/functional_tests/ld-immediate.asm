    ld a, 0x0A
    cmp a, 0x0A
    bne TEST_FAIL
    
    ld b, 0x0B
    cmp b, 0x0B
    bne TEST_FAIL

    ld c, 0x0C
    cmp c, 0x0C
    bne TEST_FAIL

    ld d, 0x0D
    cmp d, 0x0D
    bne TEST_FAIL

    ld e, 0x0E
    cmp e, 0x0E
    bne TEST_FAIL

    ld f, 0x0F
    cmp f, 0x0F
    bne TEST_FAIL

    ld g, 0x10
    cmp g, 0x10
    bne TEST_FAIL

    ld h, 0x11
    cmp h, 0x11
    bne TEST_FAIL

    ld ab, 0xABCD
    cmp ab, 0xABCD
    bne TEST_FAIL

    ld cd, 0xCDEF
    cmp cd, 0xCDEF
    bne TEST_FAIL

    ld ef, 0x0123
    cmp ef, 0x0123
    bne TEST_FAIL

    ld gh, 0x4567
    cmp gh, 0x4567
    bne TEST_FAIL

    ld sp, 0x89AB
    cmp sp, 0x89AB
    bne TEST_FAIL

    include lib/test.asm
