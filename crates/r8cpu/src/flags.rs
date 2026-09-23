/// The state of the CPU's flag bits.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Flags {
    /// Indicates carry (from addition) or 'no borrow' (from subtraction or comparison).
    pub carry: bool,
    /// Indicates a negative result from the last operation.
    pub negative: bool,
    /// Indicates a zero result from the last operation.
    pub zero: bool,
}

impl From<u8> for Flags {
    fn from(value: u8) -> Self {
        Self {
            carry: value & 0x01 != 0,
            negative: value & 0x80 != 0,
            zero: value & 0x02 != 0,
        }
    }
}

impl From<Flags> for u8 {
    fn from(flags: Flags) -> Self {
        let mut value = 0x00;
        value |= u8::from(flags.carry);
        value |= u8::from(flags.negative).strict_shl(7);
        value |= u8::from(flags.zero).strict_shl(1);
        value
    }
}

impl Flags {
    /// Updates the negative and zero flags based on `value`.
    pub fn update(&mut self, value: u8) {
        self.negative = value & 0x80 != 0;
        self.zero = value == 0;
    }

    /// Updates the negative and zero flags based on 16-bit `value`.
    pub fn update16(&mut self, value: u16) {
        self.negative = value & 0x8000 != 0;
        self.zero = value == 0;
    }
}
