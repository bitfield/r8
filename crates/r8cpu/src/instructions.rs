use core::fmt::{Display, Formatter};

use anyhow::bail;

use crate::regs::Reg;

/// Instruction kinds.
#[derive(Copy, Clone, Debug)]
pub enum InstructionKind {
    /// Add immediate.
    Add(Reg),
    /// Add register.
    AddReg,
    /// And immediate.
    And(Reg),
    /// And register.
    AndReg,
    /// Branch always.
    BranchAlways,
    /// Branch if the carry flag is clear.
    BranchCc,
    /// Branch if the carry flag is set.
    BranchCs,
    /// Branch if the zero flag is set.
    BranchEq,
    /// Branch if the negative flag is set.
    BranchMi,
    /// Branch if the zero flag is clear.
    BranchNe,
    /// Branch if the negative flag is clear.
    BranchPl,
    /// Call a subroutine.
    Call,
    /// Clear carry flag.
    Clc,
    /// Compare immediate.
    Cmp(Reg),
    /// Compare register.
    CmpReg,
    /// Decrement a register.
    Dec(Reg),
    /// Decrement a memory location in a register.
    DecIndirect,
    /// Decrement a memory location.
    DecMem,
    /// Halt the CPU.
    Halt,
    /// Increment a register.
    Inc(Reg),
    /// Increment a memory location in a register.
    IncIndirect,
    /// Increment a memory location.
    IncMem,
    /// Far jump.
    Jmp,
    /// Load a register with an immediate operand.
    Ld(Reg),
    /// Load a register from an indirect address in another register, indexed by an
    /// immediate value.
    LdIndexed,
    /// Load a register from an indirect address in another register.
    LdIndirect,
    /// Load a register from another register.
    LdReg,
    /// Logical shift right immediate.
    Lsr,
    /// Logical shift right register.
    LsrReg,
    /// No operation.
    Nop,
    /// Pop a register value from the stack.
    Pop(Reg),
    /// Pop the status register from the stack.
    PopPS,
    /// Push a register value to the stack.
    Push(Reg),
    /// Push the status register to the stack.
    PushPS,
    /// Return from a subroutine call.
    Ret,
    /// Return from a trap.
    Rti,
    /// Set carry flag.
    Sec,
    /// Store a register value at an immediate address.
    Store(Reg),
    /// Store a register at an indirect address in another register, indexed by an
    /// immediate value.
    StoreIndexed,
    /// Store a register value at an indirect address in another register.
    StoreIndirect,
    /// Store an immediate value at an indirect address in a register.
    StoreIndirectImm,
    /// Subtract immediate.
    Sub(Reg),
    /// Subtract register.
    SubReg,
    /// Test bits immediate.
    Test(Reg),
    /// Test bits register.
    TestReg,
    /// Trap with a specified code.
    Trap,
}

use InstructionKind::*;

impl Display for InstructionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Add(reg) => format!("add {reg}, N"),
                AddReg => "add R1, R2".to_owned(),
                And(reg) => format!("and {reg}, N"),
                AndReg => "and R1, R2".to_owned(),
                BranchAlways => "bra D".to_owned(),
                BranchCc => "bcc D".to_owned(),
                BranchCs => "bcs D".to_owned(),
                BranchEq => "beq D".to_owned(),
                BranchMi => "bmi D".to_owned(),
                BranchNe => "bne D".to_owned(),
                BranchPl => "bpl D".to_owned(),
                Call => "call NN".to_owned(),
                Clc => "clc".to_owned(),
                Cmp(reg) => format!("cmp {reg}, N"),
                CmpReg => "cmp R1, R2".to_owned(),
                Dec(reg) => format!("dec {reg}"),
                DecIndirect => "dec (RR)".to_owned(),
                DecMem => "dec (NN)".to_owned(),
                Halt => "halt".to_owned(),
                Inc(reg) => format!("inc {reg}"),
                IncIndirect => "inc (RR)".to_owned(),
                IncMem => "inc (NN)".to_owned(),
                Jmp => "jmp NN".to_owned(),
                LdIndexed => "ld R, (RR+N)".to_owned(),
                Ld(reg) => format!("ld {reg}, {}", if reg.is16() { "NN" } else { "N" }),
                LdIndirect => "ld R, (RR)".to_owned(),
                LdReg => "ld R1, R2".to_owned(),
                Lsr => "lsr R, S".to_owned(),
                LsrReg => "lsr R1, R2".to_owned(),
                Nop => "nop".to_owned(),
                Pop(reg) => format!("pop {reg}"),
                PopPS => "pop ps".to_owned(),
                Push(reg) => format!("push {reg}"),
                PushPS => "push ps".to_owned(),
                Ret => "ret".to_owned(),
                Rti => "rti".to_owned(),
                Sec => "sec".to_owned(),
                Store(reg) => format!("ld NN, {reg}"),
                StoreIndexed => "ld (RR+N), R".to_owned(),
                StoreIndirect => "ld (RR), R".to_owned(),
                StoreIndirectImm => "ld (RR), N".to_owned(),
                Sub(reg) => format!("sub {reg}, N"),
                SubReg => "sub R1, R2".to_owned(),
                Test(reg) => format!("test {reg}, N"),
                TestReg => "test R1, R2".to_owned(),
                Trap => "trap T".to_owned(),
            }
        )
    }
}

impl TryFrom<u8> for InstructionKind {
    type Error = anyhow::Error;

    fn try_from(opcode: u8) -> Result<Self, Self::Error> {
        let reg = Reg::try_from(opcode & 0x0F);
        Ok(match opcode {
            0x00 => Halt,
            0x01 => Nop,
            0x03 => Sec,
            0x04 => Clc,
            0x08 => Ret,
            0x09 => Rti,
            0x10..=0x1C => Ld(reg?),
            0x1D => LdIndirect,
            0x1E => LdReg,
            0x1F => LdIndexed,
            0x20..=0x27 => Store(reg?),
            0x28 => StoreIndirect,
            0x29 => StoreIndirectImm,
            0x2F => StoreIndexed,
            0x30..=0x3C => Inc(reg?),
            0x3D => IncIndirect,
            0x3E => IncMem,
            0x40..=0x4C => Dec(reg?),
            0x4D => DecIndirect,
            0x4E => DecMem,
            0x50..=0x5C => Add(reg?),
            0x5F => AddReg,
            0x60..=0x6C => Sub(reg?),
            0x6F => SubReg,
            0x70..=0x7C => Cmp(reg?),
            0x7F => CmpReg,
            0x80..=0x8C => And(reg?),
            0x8F => AndReg,
            0x90..=0x9C => Test(reg?),
            0x9F => TestReg,
            0xC2 => Lsr,
            0xC3 => LsrReg,
            0xD0..=0xDB => Push(reg?),
            0xDC => PushPS,
            0xE0..=0xEB => Pop(reg?),
            0xEC => PopPS,
            0xF0 => BranchAlways,
            0xF1 => BranchEq,
            0xF2 => BranchNe,
            0xF3 => BranchCs,
            0xF4 => BranchCc,
            0xF5 => BranchMi,
            0xF6 => BranchPl,
            0xF7 => Jmp,
            0xF8 => Call,
            0xF9 => Trap,
            _ => bail!("invalid opcode {opcode}"),
        })
    }
}

impl From<InstructionKind> for u8 {
    fn from(ins: InstructionKind) -> Self {
        match ins {
            Add(reg) => 0x50 | u8::from(reg),
            AddReg => 0x5F,
            And(reg) => 0x80 | u8::from(reg),
            AndReg => 0x8F,
            BranchAlways => 0xF0,
            BranchCc => 0xF4,
            BranchCs => 0xF3,
            BranchEq => 0xF1,
            BranchMi => 0xF5,
            BranchNe => 0xF2,
            BranchPl => 0xF6,
            Call => 0xF8,
            Clc => 0x04,
            Cmp(reg) => 0x70 | u8::from(reg),
            CmpReg => 0x7F,
            Dec(reg) => 0x40 | u8::from(reg),
            DecIndirect => 0x4D,
            DecMem => 0x4E,
            Halt => 0x00,
            Jmp => 0xF7,
            Inc(reg) => 0x30 | u8::from(reg),
            IncIndirect => 0x3D,
            IncMem => 0x3E,
            LdIndexed => 0x1F,
            Ld(reg) => 0x10 | u8::from(reg),
            LdIndirect => 0x1D,
            LdReg => 0x1E,
            Lsr => 0xC2,
            LsrReg => 0xC3,
            Nop => 0x01,
            Pop(reg) => 0xE0 | u8::from(reg),
            PopPS => 0xEC,
            Push(reg) => 0xD0 | u8::from(reg),
            PushPS => 0xDC,
            Ret => 0x08,
            Rti => 0x09,
            Sec => 0x03,
            Store(reg) => 0x20 | u8::from(reg),
            StoreIndexed => 0x2F,
            StoreIndirect => 0x28,
            StoreIndirectImm => 0x29,
            Sub(reg) => 0x60 | u8::from(reg),
            SubReg => 0x6F,
            Test(reg) => 0x90 | u8::from(reg),
            TestReg => 0x9F,
            Trap => 0xF9,
        }
    }
}

impl InstructionKind {
    /// Returns the number of operands this instruction takes.
    #[must_use]
    pub fn operands(&self) -> Operands {
        use Operands::*;
        match *self {
            Clc | Dec(_) | Halt | Inc(_) | Nop | Pop(_) | PopPS | Push(_) | PushPS | Ret | Rti
            | Sec => Zero,
            AddReg | AndReg | BranchAlways | BranchCc | BranchCs | BranchEq | BranchMi
            | BranchNe | BranchPl | CmpReg | DecIndirect | IncIndirect | LdIndirect | LdReg
            | Lsr | LsrReg | StoreIndirect | SubReg | TestReg | Trap => One,
            Add(reg) | And(reg) | Cmp(reg) | Ld(reg) | Sub(reg) | Test(reg) => {
                if reg.is16() {
                    Two
                } else {
                    One
                }
            }
            Call | DecMem | IncMem | Jmp | LdIndexed | Store(_) | StoreIndexed
            | StoreIndirectImm => Two,
        }
    }
}

/// Specifies whether an instruction takes zero, one, or two operands.
#[derive(Clone, Debug, PartialEq)]
pub enum Operands {
    /// One operand.
    One,
    /// Two operands.
    Two,
    /// Zero operands.
    Zero,
}
