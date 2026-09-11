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
    /// Reads the high byte of the address to jump to.
    ReadAddrHi,
    /// Reads a byte from memory for a `dec (NN)` instruction.
    ReadDec(u16),
    /// Reads a byte from memory for an `inc (NN)` instruction.
    ReadInc(u16),
    /// Loads a register from the bus.
    ReadLoad(Reg),
    /// Reads a single operand from the bus.
    ReadOp,
    /// Reads the second of two operands from the bus.
    ReadOpHi,
    /// Reads the first of two operands from the bus.
    ReadOpLo,
    /// Reads the new contents of the PS register from the bus.
    ReadPS,
    /// Reads the low byte of the reset vector from the bus.
    ReadResetLo,
    /// Reads the high byte of the return address for a `ret` instruction.
    ReadRetHi,
    /// Reads the low byte of the return address for a `ret` instruction.
    ReadRetLo,
    /// Reads the high byte of a stack value from the bus.
    ReadStackHi(Reg),
    /// Reads the low byte of the selected trap vector.
    ReadTrapVecLo(u16),
    /// Waits for the high byte of the address to jump to.
    WaitAddrHi,
    /// Waits for the low byte of the return address to be pushed for a `call NN`
    /// instruction.
    WaitCall(u8, u16),
    /// Waits for a byte from memory for a `dec (NN)` instruction.
    WaitDec(u16),
    /// Waits for a byte from memory for an `inc (NN)` instruction.
    WaitInc(u16),
    /// Waits for a byte from memory to load a register.
    WaitLoad(Reg),
    /// Waits for a byte from the stack to load PS.
    WaitLoadPS,
    /// Waits for a single operand read from memory.
    WaitOp,
    /// Waits for the second of two operands from memory.
    WaitOpHi,
    /// Waits for the first of two operands from memory.
    WaitOpLo,
    /// Waits for an opcode fetch to complete.
    WaitOpcode,
    /// Waits for a stack push, before pushing another value.
    WaitPush(u8),
    /// Waits for the low byte of the reset vector.
    WaitResetLo,
    /// Waits for the high byte of the return address for a `ret` instruction.
    WaitRetHi,
    /// Waits for the low byte of the return address for a `ret` instruction.
    WaitRetLo,
    /// Waits for the first of 2 stack pops to a register.
    WaitStackHi(Reg),
    /// Waits for the trap code to be pushed following a trap.
    WaitTrapCode(u8),
    /// Waits for the high byte of the return address to be pushed following a trap.
    WaitTrapHi(u8),
    /// Waits for the low byte of the return address to be pushed following a trap.
    WaitTrapLo(u8, u8),
    /// Waits for the low byte of the trap vector following a trap.
    WaitTrapVecLo(u16),
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
                ReadAddrHi => "RDAH",
                ReadDec(_) => "RDEC",
                ReadInc(_) => "RINC",
                ReadLoad(_) => "RDLD",
                ReadOp => "RDOP",
                ReadOpHi => "ROPH",
                ReadOpLo => "ROPL",
                ReadPS => "RDPS",
                ReadResetLo => "RRSL",
                ReadRetHi => "RRTH",
                ReadRetLo => "RRTL",
                ReadStackHi(_) => "RSTH",
                ReadTrapVecLo(_) => "RTVL",
                WaitCall(_, _) => "WCAL",
                WaitDec(_) => "WDEC",
                WaitInc(_) => "WINC",
                WaitLoad(_) => "WTLD",
                WaitOp => "WTOP",
                WaitOpHi => "WOPH",
                WaitOpLo => "WOPL",
                WaitOpcode => "WOPC",
                WaitLoadPS => "WTPS",
                WaitPush(_) => "WPSH",
                WaitAddrHi => "WTAH",
                WaitResetLo => "WRSL",
                WaitRetHi => "WRTH",
                WaitRetLo => "WRTL",
                WaitStackHi(_) => "WSTH",
                WaitTrapCode(_) => "WTTC",
                WaitTrapHi(_) => "WTTH",
                WaitTrapLo(_, _) => "WTTL",
                WaitTrapVecLo(_) => "WTVL",
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
                ReadAddrHi => "ReadAddrHi",
                ReadDec(_) => "ReadDec",
                ReadInc(_) => "ReadInc",
                ReadLoad(_) => "ReadLoad",
                ReadOp => "ReadOp",
                ReadOpHi => "ReadOpHi",
                ReadOpLo => "ReadOpLo",
                ReadPS => "ReadPS",
                ReadResetLo => "ReadResetLo",
                ReadRetHi => "ReadRetHi",
                ReadRetLo => "ReadRetLo",
                ReadStackHi(_) => "ReadStackHi",
                ReadTrapVecLo(_) => "ReadTrapVecLo",
                WaitCall(_, _) => "WaitCall",
                WaitDec(_) => "WaitDec",
                WaitInc(_) => "WaitInc",
                WaitLoad(_) => "WaitLoad",
                WaitOp => "WaitOp",
                WaitOpHi => "WaitOpHi",
                WaitOpLo => "WaitOpLo",
                WaitOpcode => "WaitOpcode",
                WaitLoadPS => "WaitLoadPS",
                WaitPush(_) => "WaitPush",
                WaitAddrHi => "WaitAddrHi",
                WaitResetLo => "WaitResetLo",
                WaitRetHi => "WaitRetHi",
                WaitRetLo => "WaitRetLo",
                WaitStackHi(_) => "WaitStackHi",
                WaitTrapCode(_) => "WaitTrapCode",
                WaitTrapHi(_) => "WaitTrapHi",
                WaitTrapLo(_, _) => "WaitTrapLo",
                WaitTrapVecLo(_) => "WaitTrapVecLo",
            }
        )
    }
}
