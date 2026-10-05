| | -0 | -1 | -2 | -3 | -4 | -5 | -6 | -7 | -8 | -9 | -A | -B | -C | -D | -E | -F |
| :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: |
| 0- | halt | nop | | sec | clc | | | | ret | rti | | | | | | |
| 1- | ld a, N | ld b, N | ld c, N | ld d, N | ld e, N | ld f, N | ld g, N | ld h, N | ld ab, NN | ld cd, NN | ld ef, NN | ld gh, NN | ld sp, NN | ld R, (RR) | ld R1, R2 | ld R, (RR+N) |
| 2- | ld NN, a | ld NN, b | ld NN, c | ld NN, d | ld NN, e | ld NN, f | ld NN, g | ld NN, h | ld (RR), R | ld (RR), N | | | | | | ld (RR+N), R |
| 3- | inc a | inc b | inc c | inc d | inc e | inc f | inc g | inc h | inc ab | inc cd | inc ef | inc gh | inc sp | inc (RR) | inc (NN) | |
| 4- | dec a | dec b | dec c | dec d | dec e | dec f | dec g | dec h | dec ab | dec cd | dec ef | dec gh | dec sp | dec (RR) | dec (NN) | |
| 5- | add a, N | add b, N | add c, N | add d, N | add e, N | add f, N | add g, N | add h, N | add ab, N | add cd, N | add ef, N | add gh, N | add sp, N | | | add R1, R2 |
| 6- | sub a, N | sub b, N | sub c, N | sub d, N | sub e, N | sub f, N | sub g, N | sub h, N | sub ab, N | sub cd, N | sub ef, N | sub gh, N | sub sp, N | | | sub R1, R2 |
| 7- | cmp a, N | cmp b, N | cmp c, N | cmp d, N | cmp e, N | cmp f, N | cmp g, N | cmp h, N | cmp ab, N | cmp cd, N | cmp ef, N | cmp gh, N | cmp sp, N | | | cmp R1, R2 |
| 8- | and a, N | and b, N | and c, N | and d, N | and e, N | and f, N | and g, N | and h, N | and ab, N | and cd, N | and ef, N | and gh, N | and sp, N | | | and R1, R2 |
| 9- | test a, N | test b, N | test c, N | test d, N | test e, N | test f, N | test g, N | test h, N | test ab, N | test cd, N | test ef, N | test gh, N | test sp, N | | | test R1, R2 |
| A- | | | | | | | | | | | | | | | | |
| B- | | | | | | | | | | | | | | | | |
| C- | | | lsr R, S | lsr R1, R2 | | | | | | | | | | | | |
| D- | push a | push b | push c | push d | push e | push f | push g | push h | push ab | push cd | push ef | push gh | push ps | | | |
| E- | pop a | pop b | pop c | pop d | pop e | pop f | pop g | pop h | pop ab | pop cd | pop ef | pop gh | pop ps | | | |
| F- | bra D | beq D | bne D | bcs D | bcc D | bmi D | bpl D | jmp NN | call NN | trap T | | | | | | |
