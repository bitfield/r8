use core::fmt::{Debug, Display, Formatter};

use r8cpu::regs::Reg;

/// The state of the CPU on the next tick.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum State {
    /// Reads the opcode from the data bus.
    Decode,
    /// Executes the current instruction.
    Execute,
    /// Requests the next opcode from memory.
    #[default]
    FetchOpcode,
    /// Waits for a stack push, before pushing another value.
    PushData(u8),
    /// Waits for the high byte of the return address to be pushed following a trap.
    PushRetHi(u8),
    /// Waits for the low byte of the return address to be pushed following a trap.
    PushRetLo(u8, u8),
    /// Waits for the trap code to be pushed following a trap.
    PushTCode(u8),
    /// Loads a register from the bus.
    ReadData(Reg),
    /// Reads a byte from memory for a `dec (NN)` instruction.
    ReadDec(u16),
    /// Reads a byte from memory for an `inc (NN)` instruction.
    ReadInc(u16),
    /// Reads a single operand from the bus.
    ReadOp,
    /// Reads the second of two operands from the bus.
    ReadOpHi,
    /// Reads the first of two operands from the bus.
    ReadOpLo,
    /// Reads the new contents of the PS register from the bus.
    ReadPS,
    /// Reads the high byte of the return address for a `ret` instruction.
    ReadRetHi,
    /// Reads the low byte of the return address for a `ret` instruction.
    ReadRetLo,
    /// Reads the high byte of a stack value from the bus.
    ReadStackHi(Reg),
    /// Reads the high byte of the selected vector.
    ReadVecHi,
    /// Reads the low byte of the selected vector.
    ReadVecLo(u16),
    /// Waits for the low byte of the return address to be pushed for a `call NN`
    /// instruction.
    WaitCall(u8, u16),
    /// Waits for a byte from memory to load a register.
    WaitData(Reg),
    /// Waits for a byte from memory for a `dec (NN)` instruction.
    WaitDec(u16),
    /// Waits for a byte from memory for an `inc (NN)` instruction.
    WaitInc(u16),
    /// Waits for a single operand read from memory.
    WaitOp,
    /// Waits for the second of two operands from memory.
    WaitOpHi,
    /// Waits for the first of two operands from memory.
    WaitOpLo,
    /// Waits for an opcode fetch to complete.
    WaitOpcode,
    /// Waits for a byte from the stack to load PS.
    WaitPS,
    /// Waits for the high byte of the return address for a `ret` instruction.
    WaitRetHi,
    /// Waits for the low byte of the return address for a `ret` instruction.
    WaitRetLo,
    /// Waits for the first of 2 stack pops to a register.
    WaitStackHi(Reg),
    /// Waits for the high byte of the vector following a trap or reset.
    WaitVecHi,
    /// Waits for the low byte of the vector following a trap or reset.
    WaitVecLo(u16),
}

impl Debug for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        use State::*;
        write!(
            f,
            "{}",
            match *self {
                Decode => "DCOD",
                Execute => "EXEC",
                FetchOpcode => "FOPC",
                ReadVecHi => "RDAH",
                ReadDec(_) => "RDEC",
                ReadInc(_) => "RINC",
                ReadData(_) => "RDLD",
                ReadOp => "RDOP",
                ReadOpHi => "ROPH",
                ReadOpLo => "ROPL",
                ReadPS => "RDPS",
                ReadRetHi => "RRTH",
                ReadRetLo => "RRTL",
                ReadStackHi(_) => "RSTH",
                ReadVecLo(_) => "RTVL",
                WaitCall(_, _) => "WCAL",
                WaitDec(_) => "WDEC",
                WaitInc(_) => "WINC",
                WaitData(_) => "WTLD",
                WaitOp => "WTOP",
                WaitOpHi => "WOPH",
                WaitOpLo => "WOPL",
                WaitOpcode => "WOPC",
                WaitPS => "WTPS",
                PushData(_) => "WPSH",
                WaitVecHi => "WTAH",
                WaitRetHi => "WRTH",
                WaitRetLo => "WRTL",
                WaitStackHi(_) => "WSTH",
                PushTCode(_) => "WTTC",
                PushRetHi(_) => "WTTH",
                PushRetLo(_, _) => "WTTL",
                WaitVecLo(_) => "WTVL",
            }
        )
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        use State::*;
        write!(
            f,
            "{}",
            match *self {
                Decode => "Decode",
                Execute => "Execute",
                FetchOpcode => "FetchOpcode",
                ReadVecHi => "ReadAddrHi",
                ReadDec(_) => "ReadDec",
                ReadInc(_) => "ReadInc",
                ReadData(_) => "ReadLoad",
                ReadOp => "ReadOp",
                ReadOpHi => "ReadOpHi",
                ReadOpLo => "ReadOpLo",
                ReadPS => "ReadPS",
                ReadRetHi => "ReadRetHi",
                ReadRetLo => "ReadRetLo",
                ReadStackHi(_) => "ReadStackHi",
                ReadVecLo(_) => "ReadTrapVecLo",
                WaitCall(_, _) => "WaitCall",
                WaitDec(_) => "WaitDec",
                WaitInc(_) => "WaitInc",
                WaitData(_) => "WaitLoad",
                WaitOp => "WaitOp",
                WaitOpHi => "WaitOpHi",
                WaitOpLo => "WaitOpLo",
                WaitOpcode => "WaitOpcode",
                WaitPS => "WaitLoadPS",
                PushData(_) => "WaitPush",
                WaitVecHi => "WaitAddrHi",
                WaitRetHi => "WaitRetHi",
                WaitRetLo => "WaitRetLo",
                WaitStackHi(_) => "WaitStackHi",
                PushTCode(_) => "WaitTrapCode",
                PushRetHi(_) => "WaitTrapHi",
                PushRetLo(_, _) => "WaitTrapLo",
                WaitVecLo(_) => "WaitTrapVecLo",
            }
        )
    }
}
