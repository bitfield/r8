#[must_use]
pub fn add(augend: u8, addend: u8, carry_in: bool) -> (u8, bool) {
    let (tmp_result, carry1) = augend.overflowing_add(addend);
    let carry_in = u8::from(carry_in);
    let (result, carry2) = tmp_result.overflowing_add(carry_in);
    let carry = carry1 || carry2;
    (result, carry)
}

#[must_use]
pub fn add16(augend: u16, addend: u16, carry_in: bool) -> (u16, bool) {
    let (tmp_result, carry1) = augend.overflowing_add(addend);
    let carry_in = u16::from(carry_in);
    let (result, carry2) = tmp_result.overflowing_add(carry_in);
    let carry = carry1 || carry2;
    (result, carry)
}

#[must_use]
pub fn and(input: u8, mask: u8) -> u8 {
    input & mask
}

#[must_use]
pub fn and16(input: u16, mask: u16) -> u16 {
    input & mask
}

#[must_use]
pub fn cmp(lhs: u8, rhs: u8) -> (u8, bool) {
    (lhs.wrapping_sub(rhs), lhs >= rhs)
}

#[must_use]
pub fn cmp16(lhs: u16, rhs: u16) -> (u16, bool) {
    (lhs.wrapping_sub(rhs), lhs >= rhs)
}

#[must_use]
pub fn dec(input: u8) -> u8 {
    input.wrapping_sub(1)
}

#[must_use]
pub fn dec16(input: u16) -> u16 {
    input.wrapping_sub(1)
}

#[must_use]
pub fn inc(input: u8) -> u8 {
    input.wrapping_add(1)
}

#[must_use]
pub fn inc16(input: u16) -> u16 {
    input.wrapping_add(1)
}

#[must_use]
pub fn lsr(input: u8, mut shift: u8) -> (u8, bool) {
    shift = shift.clamp(1, 8);
    let value = input.unbounded_shr(u32::from(shift.strict_sub(1))); // clamped >= 1
    let carry = (value & 1) != 0;
    let result = value.unbounded_shr(1);
    (result, carry)
}

#[must_use]
pub fn lsr16(input: u16, mut shift: u8) -> (u16, bool) {
    shift = shift.clamp(1, 16);
    let value = input.unbounded_shr(u32::from(shift.strict_sub(1))); // clamped >= 1
    let carry = (value & 1) != 0;
    let result = value.unbounded_shr(1);
    (result, carry)
}

#[must_use]
pub fn shl(input: u8, mut shift: u8) -> (u8, bool) {
    shift = shift.clamp(1, 8);
    let value = input.unbounded_shl(u32::from(shift.strict_sub(1))); // clamped >= 1
    let carry = (value & 0x80) != 0;
    let result = value.unbounded_shl(1);
    (result, carry)
}

#[must_use]
pub fn shl16(input: u16, mut shift: u8) -> (u16, bool) {
    shift = shift.clamp(1, 16);
    let value = input.unbounded_shl(u32::from(shift.strict_sub(1))); // clamped >= 1
    let carry = (value & 0x8000) != 0;
    let result = value.unbounded_shl(1);
    (result, carry)
}

#[must_use]
pub fn sub(minuend: u8, subtrahend: u8, carry_in: bool) -> (u8, bool) {
    let (tmp_result, borrow1) = minuend.overflowing_sub(subtrahend);
    let borrow_in = u8::from(!carry_in);
    let (result, borrow2) = tmp_result.overflowing_sub(borrow_in);
    let carry_out = !(borrow1 || borrow2);
    (result, carry_out)
}

#[must_use]
pub fn sub16(minuend: u16, subtrahend: u16, carry_in: bool) -> (u16, bool) {
    let (tmp_result, borrow1) = minuend.overflowing_sub(subtrahend);
    let borrow_in = u16::from(!carry_in);
    let (result, borrow2) = tmp_result.overflowing_sub(borrow_in);
    let carry_out = !(borrow1 || borrow2);
    (result, carry_out)
}
