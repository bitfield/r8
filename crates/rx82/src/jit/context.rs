// SPDX-FileCopyrightText: 2026 Zayn Otley
// https://ZaynOtley.com
// https://IntuitionEngine.io
// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(clippy::single_call_fn, reason = "keep conversion helpers small")]
use super::ffi;

use crate::{bus::Bus, cpu::Cpu, state::State};
use r8cpu::regs::Reg;

macro_rules! state_codec {
    (plain [$($plain:ident => $ptag:ident),*];
     byte [$($byte:ident => $btag:ident),*];
     address [$($addr:ident => $atag:ident),*];
     register [$($reg:ident => $rtag:ident),*]) => {
        #[expect(clippy::cast_possible_truncation, reason = "C state tags are constants below 34")]
        fn encode_state(state: State) -> (u16, u16, u8, u8) {
            match state {
                $(State::$plain => (ffi::$ptag as u16, 0, 0, 0),)*
                $(State::$byte(value) => (ffi::$btag as u16, 0, value, 0),)*
                $(State::$addr(value) => (ffi::$atag as u16, value, 0, 0),)*
                $(State::$reg(value) => (ffi::$rtag as u16, 0, u8::from(value), 0),)*
                State::PushRetLo(lo, trap) => (ffi::R8_PUSH_RET_LO as u16, 0, lo, trap),
                State::WaitCall(lo, addr) => (ffi::R8_WAIT_CALL as u16, addr, lo, 0),
            }
        }

        pub(super) fn decode_state(context: &ffi::r8_context) -> Option<State> {
            Some(match u32::from(context.state) {
                $(ffi::$ptag => State::$plain,)*
                $(ffi::$btag => State::$byte(context.arg0),)*
                $(ffi::$atag => State::$addr(context.arg16),)*
                $(ffi::$rtag => State::$reg(Reg::try_from(context.arg0).ok()?),)*
                ffi::R8_PUSH_RET_LO => State::PushRetLo(context.arg0, context.arg1),
                ffi::R8_WAIT_CALL => State::WaitCall(context.arg0, context.arg16),
                _ => return None,
            })
        }
    };
}

state_codec! {
    plain [Decode => R8_DECODE, Execute => R8_EXECUTE, FetchOpcode => R8_FETCH_OPCODE,
        ReadFlags => R8_READ_FLAGS, ReadOp => R8_READ_OP, ReadOpHi => R8_READ_OP_HI,
        ReadOpLo => R8_READ_OP_LO, ReadPS => R8_READ_PS, ReadRetLo => R8_READ_RET_LO,
        ReadVecHi => R8_READ_VEC_HI, WaitFlags => R8_WAIT_FLAGS, WaitOp => R8_WAIT_OP,
        WaitOpHi => R8_WAIT_OP_HI, WaitOpLo => R8_WAIT_OP_LO, WaitOpcode => R8_WAIT_OPCODE,
        WaitPS => R8_WAIT_PS, WaitRetLo => R8_WAIT_RET_LO, WaitVecHi => R8_WAIT_VEC_HI];
    byte [PushData => R8_PUSH_DATA, PushFlags => R8_PUSH_FLAGS, PushTrap => R8_PUSH_TRAP];
    address [ReadDec => R8_READ_DEC, ReadInc => R8_READ_INC, ReadVecLo => R8_READ_VEC_LO,
        ReqVecLo => R8_REQ_VEC_LO, WaitDec => R8_WAIT_DEC, WaitInc => R8_WAIT_INC,
        WaitVecLo => R8_WAIT_VEC_LO];
    register [ReadData => R8_READ_DATA, ReadStackHi => R8_READ_STACK_HI,
        WaitData => R8_WAIT_DATA, WaitStackHi => R8_WAIT_STACK_HI]
}

pub(super) fn encode(cpu: &Cpu, bus: &Bus) -> ffi::r8_context {
    let (state, arg16, arg0, arg1) = encode_state(cpu.state);
    ffi::r8_context {
        regs: cpu.regs.bytes(),
        sp: cpu.regs.get16(Reg::SP),
        pc: cpu.pc,
        flags: u8::from(cpu.flags),
        halt: u8::from(cpu.halt),
        ins: u8::from(cpu.ins),
        op_lo: cpu.op_lo,
        op_hi: cpu.op_hi,
        state,
        arg16,
        arg0,
        arg1,
        vendor: *cpu.id.vendor,
        capabilities: u8::from(cpu.id.capabilities),
        extended: cpu.id.extended,
        family: cpu.id.family,
        highest: cpu.id.highest,
        model: cpu.id.model,
        revision: cpu.id.revision,
        bus_data: bus.data,
        ..Default::default()
    }
}

pub(super) fn apply(context: &ffi::r8_context, cpu: &mut Cpu, bus: &mut Bus) -> bool {
    let Some(state) = decode_state(context) else {
        return false;
    };
    if context.request > 4 || context.halt > 1 || context.host_output > 1 {
        return false;
    }
    if context.host_output != 0 {
        print!("{}", char::from(context.regs[0]));
    }
    cpu.regs.set_bytes(context.regs);
    cpu.regs.set16(Reg::SP, context.sp);
    cpu.pc = context.pc;
    cpu.flags = context.flags.into();
    cpu.halt = context.halt != 0;
    cpu.op_lo = context.op_lo;
    cpu.op_hi = context.op_hi;
    cpu.state = state;
    match context.request {
        1 => bus.disable_mem(),
        2 => bus.read_mem(context.request_addr),
        3 => bus.write_data(context.request_data),
        4 => bus.write_mem(context.request_addr, context.request_data),
        _ => (),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use r8cpu::regs::Reg;

    #[test]
    fn every_continuation_round_trips_with_payloads() {
        use State::*;
        let states = [
            Decode,
            Execute,
            FetchOpcode,
            PushData(255),
            PushFlags(255),
            PushRetLo(128, 255),
            PushTrap(255),
            ReadData(Reg::SP),
            ReadDec(0xFFFF),
            ReadFlags,
            ReadInc(0xFFFF),
            ReadOp,
            ReadOpHi,
            ReadOpLo,
            ReadPS,
            ReadRetLo,
            ReadStackHi(Reg::GH),
            ReadVecHi,
            ReadVecLo(0xFFFF),
            ReqVecLo(0xFFFF),
            WaitCall(255, 0xFFFF),
            WaitData(Reg::H),
            WaitDec(0xFFFF),
            WaitFlags,
            WaitInc(0xFFFF),
            WaitOp,
            WaitOpHi,
            WaitOpLo,
            WaitOpcode,
            WaitPS,
            WaitRetLo,
            WaitStackHi(Reg::AB),
            WaitVecHi,
            WaitVecLo(0xFFFF),
        ];
        for state in states {
            let cpu = Cpu {
                state,
                ..Cpu::default()
            };
            let context = encode(&cpu, &Bus::default());
            assert_eq!(decode_state(&context), Some(state));
        }
    }
}
