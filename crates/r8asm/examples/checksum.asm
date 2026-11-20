START:
    ld cd, START
    ld a, 0x00
LOOP:
    ld b, (cd)
    add a, b
    cmp cd, END
    beq END
    inc cd
    jmp LOOP
END:
    halt
