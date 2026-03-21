use thiserror::Error;

use crate::core::bus::{CpuBus, InterruptLines};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepRecord {
    pub pc_before: u16,
    pub opcode: u8,
    pub bytes: [u8; 3],
    pub byte_len: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub status: u8,
    pub sp: u8,
    pub cyc_before: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CpuError {
    #[error("CPU execution is not implemented yet")]
    Unimplemented,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub sp: u8,
    pub status: u8,
    pub total_cycles: u64,
    pub instruction_count: u64,
    pub pending_interrupts: InterruptLines,
}

impl Cpu {
    pub fn reset(&mut self, bus: &mut impl CpuBus) {
        let lo = bus.read(0xFFFC) as u16;
        let hi = bus.read(0xFFFD) as u16;
        self.pc = (hi << 8) | lo;
        self.sp = 0xFD;
        self.status = 0x24;
        self.pending_interrupts = bus.interrupt_lines();
    }

    pub fn step_instruction(&mut self, bus: &mut impl CpuBus) -> Result<StepRecord, CpuError> {
        let _record = StepRecord {
            pc_before: self.pc,
            opcode: bus.read(self.pc),
            bytes: [bus.read(self.pc), bus.read(self.pc.wrapping_add(1)), bus.read(self.pc.wrapping_add(2))],
            byte_len: 1,
            a: self.a,
            x: self.x,
            y: self.y,
            status: self.status,
            sp: self.sp,
            cyc_before: self.total_cycles,
        };

        Err(CpuError::Unimplemented)
    }
}