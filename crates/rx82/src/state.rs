use core::fmt::{Debug, Display, Formatter};

use r8cpu::regs::Reg;

/// The state of the CPU on the next tick.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum State {
    /// Decodes the opcode from the data bus.
    Decode,
    /// Executes the current instruction.
    Execute,
    /// Requests the next opcode from memory.
    #[default]
    FetchOpcode,
    /// Waits for a stack push, before pushing another value.
    PushData(u8),
    /// Pushes the `ps` register following a trap.
    PushFlags(u8),
    /// Pushes the low byte of the return address following a trap.
    PushRetLo(u8, u8),
    /// Pushes the trap code following a trap.
    PushTrap(u8),
    /// Loads a register from the bus.
    ReadData(Reg),
    /// Reads a byte from memory for a `dec (NN)` instruction.
    ReadDec(u16),
    /// Reads the `ps` register from the stack.
    ReadFlags,
    /// Reads a byte from memory for an `inc (NN)` instruction.
    ReadInc(u16),
    /// Reads a single operand from the bus.
    ReadOp,
    /// Reads the second of two operands from the bus.
    ReadOpHi,
    /// Reads the first of two operands from the bus.
    ReadOpLo,
    /// Reads the new contents of the `ps` register from the bus.
    ReadPS,
    /// Reads the low byte of the return address for a `ret` / `rti` instruction.
    ReadRetLo,
    /// Reads the high byte of a stack value from the bus.
    ReadStackHi(Reg),
    /// Reads the high byte of the desired vector.
    ReadVecHi,
    /// Reads the low byte of the desired vector.
    ReadVecLo(u16),
    /// Requests the low byte of a trap vector.
    ReqVecLo(u16),
    /// Waits for the high byte of the return address to be pushed for a `call NN`
    /// instruction.
    WaitCall(u8, u16),
    /// Waits for a byte from memory to load a register.
    WaitData(Reg),
    /// Waits for a byte from memory for a `dec (NN)` instruction.
    WaitDec(u16),
    /// Waits for the `ps` register to be read from the stack.
    WaitFlags,
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
    /// Waits for a byte from the stack to load `ps`.
    WaitPS,
    /// Waits for the low byte of the return address for a `ret` / `rti` instruction.
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
                PushData(_) => "WPSH",
                PushFlags(_) => "PFLG",
                PushRetLo(_, _) => "PRTL",
                PushTrap(_) => "PTRP",
                ReadData(_) => "RDLD",
                ReadDec(_) => "RDEC",
                ReadFlags => "RFLG",
                ReadInc(_) => "RINC",
                ReadOp => "RDOP",
                ReadOpHi => "ROPH",
                ReadOpLo => "ROPL",
                ReadPS => "RDPS",
                ReadRetLo => "RRTL",
                ReadStackHi(_) => "RSTH",
                ReadVecHi => "RDAH",
                ReadVecLo(_) => "RTVL",
                ReqVecLo(_) => "RQVL",
                WaitCall(_, _) => "WCAL",
                WaitData(_) => "WTLD",
                WaitDec(_) => "WDEC",
                WaitFlags => "WFLG",
                WaitInc(_) => "WINC",
                WaitOp => "WTOP",
                WaitOpHi => "WOPH",
                WaitOpLo => "WOPL",
                WaitOpcode => "WOPC",
                WaitPS => "WTPS",
                WaitRetLo => "WRTL",
                WaitStackHi(_) => "WSTH",
                WaitVecHi => "WTAH",
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
                PushData(_) => "PushData",
                PushFlags(_) => "PushFlags",
                PushRetLo(_, _) => "PushRetLo",
                PushTrap(_) => "PushTrap",
                ReadData(_) => "ReadData",
                ReadDec(_) => "ReadDec",
                ReadFlags => "ReadFlags",
                ReadInc(_) => "ReadInc",
                ReadOp => "ReadOp",
                ReadOpHi => "ReadOpHi",
                ReadOpLo => "ReadOpLo",
                ReadPS => "ReadPS",
                ReadRetLo => "ReadRetLo",
                ReadStackHi(_) => "ReadStackHi",
                ReadVecHi => "ReadVecHi",
                ReadVecLo(_) => "ReadVecLo",
                ReqVecLo(_) => "ReqVecLo",
                WaitCall(_, _) => "WaitCall",
                WaitData(_) => "WaitData",
                WaitDec(_) => "WaitDec",
                WaitFlags => "WaitFlags",
                WaitInc(_) => "WaitInc",
                WaitOp => "WaitOp",
                WaitOpHi => "WaitOpHi",
                WaitOpLo => "WaitOpLo",
                WaitOpcode => "WaitOpcode",
                WaitPS => "WaitPS",
                WaitRetLo => "WaitRetLo",
                WaitStackHi(_) => "WaitStackHi",
                WaitVecHi => "WaitVecHi",
                WaitVecLo(_) => "WaitVecLo",
            }
        )
    }
}
