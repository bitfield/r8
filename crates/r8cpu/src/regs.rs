use anyhow::bail;

use core::{
    fmt::{Display, Formatter},
    str::FromStr,
};

/// The 8-bit registers.
#[expect(clippy::min_ident_chars, reason = "the actual names")]
#[expect(clippy::arbitrary_source_item_ordering, reason = "logical order")]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Reg {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    AB,
    CD,
    EF,
    GH,
    SP,
}

impl Display for Reg {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Reg::A => "a",
                Reg::B => "b",
                Reg::C => "c",
                Reg::D => "d",
                Reg::E => "e",
                Reg::F => "f",
                Reg::G => "g",
                Reg::H => "h",
                Reg::AB => "ab",
                Reg::CD => "cd",
                Reg::EF => "ef",
                Reg::GH => "gh",
                Reg::SP => "sp",
            }
        )
    }
}

impl FromStr for Reg {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, anyhow::Error> {
        Ok(match value {
            "a" => Reg::A,
            "b" => Reg::B,
            "c" => Reg::C,
            "d" => Reg::D,
            "e" => Reg::E,
            "f" => Reg::F,
            "g" => Reg::G,
            "h" => Reg::H,
            "ab" => Reg::AB,
            "cd" => Reg::CD,
            "ef" => Reg::EF,
            "gh" => Reg::GH,
            "sp" => Reg::SP,
            reg => bail!("invalid register {reg}"),
        })
    }
}

impl TryFrom<u8> for Reg {
    type Error = anyhow::Error;

    fn try_from(id: u8) -> Result<Self, Self::Error> {
        Ok(match id {
            0x00 => Reg::A,
            0x01 => Reg::B,
            0x02 => Reg::C,
            0x03 => Reg::D,
            0x04 => Reg::E,
            0x05 => Reg::F,
            0x06 => Reg::G,
            0x07 => Reg::H,
            0x08 => Reg::AB,
            0x09 => Reg::CD,
            0x0A => Reg::EF,
            0x0B => Reg::GH,
            0x0C => Reg::SP,
            _ => bail!("invalid register id {id:#04X}"),
        })
    }
}

impl From<Reg> for u8 {
    fn from(reg: Reg) -> Self {
        match reg {
            Reg::A => 0x00,
            Reg::B => 0x01,
            Reg::C => 0x02,
            Reg::D => 0x03,
            Reg::E => 0x04,
            Reg::F => 0x05,
            Reg::G => 0x06,
            Reg::H => 0x07,
            Reg::AB => 0x08,
            Reg::CD => 0x09,
            Reg::EF => 0x0A,
            Reg::GH => 0x0B,
            Reg::SP => 0x0C,
        }
    }
}

impl Reg {
    /// Returns true if `reg` is a 16-bit register pair.
    #[must_use]
    pub fn is16(&self) -> bool {
        matches!(self, Reg::AB | Reg::CD | Reg::EF | Reg::GH | Reg::SP)
    }
}

pub struct RegToReg {
    pub source: Reg,
    pub target: Reg,
}

impl From<RegToReg> for u8 {
    fn from(input: RegToReg) -> Self {
        (u8::from(input.source) << 4_u8) | u8::from(input.target)
    }
}

impl TryFrom<u8> for RegToReg {
    type Error = anyhow::Error;

    fn try_from(encoded_regs: u8) -> Result<Self, Self::Error> {
        if let Ok(source) = Reg::try_from(encoded_regs.strict_shr(4))
            && let Ok(target) = Reg::try_from(encoded_regs & 0x0F)
        {
            Ok(Self { source, target })
        } else {
            bail!("invalid register id {encoded_regs:#04X}")
        }
    }
}

/// The CPU registers.
#[derive(Debug, Default)]
pub struct Regs {
    ra: u8,
    rb: u8,
    rc: u8,
    rd: u8,
    re: u8,
    rf: u8,
    rg: u8,
    rh: u8,
    sp: u16,
}

impl Regs {
    /// Returns the value in register `reg`.
    #[must_use]
    pub fn get(&self, reg: Reg) -> u8 {
        use Reg::*;
        match reg {
            A => self.ra,
            B => self.rb,
            C => self.rc,
            D => self.rd,
            E => self.re,
            F => self.rf,
            G => self.rg,
            H => self.rh,
            other => unreachable!("get() called with 16-bit register pair '{other}'"),
        }
    }

    /// Returns the value in register pair `reg`.
    #[must_use]
    pub fn get16(&self, reg: Reg) -> u16 {
        use Reg::*;
        match reg {
            AB => u16::from_be_bytes([self.ra, self.rb]),
            CD => u16::from_be_bytes([self.rc, self.rd]),
            EF => u16::from_be_bytes([self.re, self.rf]),
            GH => u16::from_be_bytes([self.rg, self.rh]),
            SP => self.sp,
            other => unreachable!("get16() called with 8-bit register '{other}'"),
        }
    }

    /// Sets register `reg` to the value `val`.
    pub fn set(&mut self, reg: Reg, value: u8) {
        use Reg::*;
        match reg {
            A => self.ra = value,
            B => self.rb = value,
            C => self.rc = value,
            D => self.rd = value,
            E => self.re = value,
            F => self.rf = value,
            G => self.rg = value,
            H => self.rh = value,
            other => unreachable!("set() called with 16-bit register pair '{other}'"),
        }
    }

    /// Sets register pair `reg` to the value `val`.
    pub fn set16(&mut self, reg: Reg, value: u16) {
        use Reg::*;
        let [hi, lo] = value.to_be_bytes();
        match reg {
            AB => [self.ra, self.rb] = [hi, lo],
            CD => [self.rc, self.rd] = [hi, lo],
            EF => [self.re, self.rf] = [hi, lo],
            GH => [self.rg, self.rh] = [hi, lo],
            SP => self.sp = value,
            other => unreachable!("set16() called with 8-bit register '{other}'"),
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests")]
mod tests {
    use crate::regs::{Reg::AB, Reg::*};

    use super::*;

    #[test]
    fn addressing_individual_regs_works() {
        let mut regs = Regs::default();
        regs.set(A, 0x00FF);
        assert_eq!(regs.get(A), 0xFF, "wrong A");
        regs.set(F, 0x00FF);
        assert_eq!(regs.get(F), 0xFF, "wrong F");
    }

    #[test]
    fn addressing_reg_pairs_works() {
        let mut regs = Regs::default();
        regs.set(A, 0x00DE);
        regs.set(B, 0x00AD);
        assert_eq!(regs.get16(AB), 0xDEAD, "wrong AB");
        regs.set16(AB, 0xBEEF);
        assert_eq!(regs.get16(AB), 0xBEEF, "wrong AB");
        assert_eq!(regs.get(A), 0xBE, "wrong A");
        assert_eq!(regs.get(B), 0xEF, "wrong B");
        regs.set16(GH, 0xCAFE);
        assert_eq!(regs.get16(GH), 0xCAFE, "wrong GH");
    }

    #[test]
    fn addressing_sp_works() {
        let mut regs = Regs::default();
        let sp = Reg::SP;
        regs.set16(sp, 0xFFFF);
        assert!(sp.is16(), "SP should be 16-bit");
        assert_eq!(regs.get16(sp), 0xFFFF, "wrong SP");
    }

    #[test]
    fn reg_display_names_work() {
        assert_eq!(format!("{}", Reg::F), "f");
    }

    #[test]
    fn reg_name_parsing_works() {
        assert_eq!("f".parse::<Reg>().unwrap(), Reg::F);
    }

    #[test]
    fn reg_marshalling_to_u8_works() {
        assert_eq!(u8::from(Reg::F), 0x05);
    }

    #[test]
    fn reg_unmarshalling_from_u8_works() {
        assert_eq!(Reg::try_from(0x05).unwrap(), Reg::F);
    }
}
