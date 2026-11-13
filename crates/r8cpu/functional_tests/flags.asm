    sec
    bcc TEST_FAIL
    
    clc
    bcs TEST_FAIL

    ld a, 0x00
    bne TEST_FAIL

    ld b, 0x01
    beq TEST_FAIL

    ld c, 0x80
    bpl TEST_FAIL

    ld d, 0x7F
    bmi TEST_FAIL
    
    include lib/test.asm
