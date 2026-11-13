TEST_PASS:
    ld a, 0x00     ; pass
    bra TEST_END
TEST_FAIL:
    ld a, 0x01     ; fail
TEST_END:
    halt
