mod addressing;
mod execute;
mod opcode;
mod trace;

use thiserror::Error;

use crate::core::bus::{CpuBus, InterruptLines};

pub use addressing::{AddressingMode, ResolvedOperand};
pub use opcode::{OPCODES, OpcodeMeta, opcode_meta};
pub use trace::{format_trace_line, format_trace_lines};

pub const STATUS_CARRY: u8 = 0b0000_0001;
pub const STATUS_ZERO: u8 = 0b0000_0010;
pub const STATUS_INTERRUPT_DISABLE: u8 = 0b0000_0100;
pub const STATUS_DECIMAL: u8 = 0b0000_1000;
pub const STATUS_BREAK: u8 = 0b0001_0000;
pub const STATUS_UNUSED: u8 = 0b0010_0000;
pub const STATUS_OVERFLOW: u8 = 0b0100_0000;
pub const STATUS_NEGATIVE: u8 = 0b1000_0000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    pub operand_addr: Option<u16>,
    pub operand_value: Option<u8>,
    pub pointer_addr: Option<u16>,
    pub pointer_value: Option<u16>,
    pub branch_target: Option<u16>,
    pub page_crossed: bool,
}

impl StepRecord {
    pub fn cycles_used(&self, total_cycles_after: u64) -> u64 {
        total_cycles_after.saturating_sub(self.cyc_before)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CpuError {
    #[error("unsupported opcode 0x{opcode:02X} at 0x{pc:04X}")]
    UnsupportedOpcode { opcode: u8, pc: u16 },
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
        self.status = STATUS_INTERRUPT_DISABLE | STATUS_UNUSED;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.total_cycles = 7;
        self.instruction_count = 0;
        self.pending_interrupts = bus.interrupt_lines();
    }

    fn stack_addr(&self) -> u16 {
        0x0100 | self.sp as u16
    }

    fn push_byte(&mut self, bus: &mut impl CpuBus, value: u8) {
        bus.write(self.stack_addr(), value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull_byte(&mut self, bus: &mut impl CpuBus) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.read(self.stack_addr())
    }

    fn push_u16(&mut self, bus: &mut impl CpuBus, value: u16) {
        self.push_byte(bus, (value >> 8) as u8);
        self.push_byte(bus, value as u8);
    }

    fn pull_u16(&mut self, bus: &mut impl CpuBus) -> u16 {
        let lo = self.pull_byte(bus) as u16;
        let hi = self.pull_byte(bus) as u16;
        (hi << 8) | lo
    }

    fn tick(&mut self, bus: &mut impl CpuBus, cycles: u8) {
        for _ in 0..cycles {
            bus.tick();
            self.total_cycles += 1;
        }
    }

    pub fn service_reset(&mut self, bus: &mut impl CpuBus) {
        self.reset(bus);
    }

    pub fn service_nmi(&mut self, bus: &mut impl CpuBus) {
        self.push_u16(bus, self.pc);
        self.push_byte(bus, self.status_for_stack(false));
        self.set_flag(STATUS_INTERRUPT_DISABLE, true);
        let lo = bus.read(0xFFFA) as u16;
        let hi = bus.read(0xFFFB) as u16;
        self.pc = (hi << 8) | lo;
        self.tick(bus, 7);
    }

    pub fn service_irq(&mut self, bus: &mut impl CpuBus) {
        self.push_u16(bus, self.pc);
        self.push_byte(bus, self.status_for_stack(false));
        self.set_flag(STATUS_INTERRUPT_DISABLE, true);
        let lo = bus.read(0xFFFE) as u16;
        let hi = bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
        self.tick(bus, 7);
    }

    pub fn service_brk(&mut self, bus: &mut impl CpuBus) {
        let return_pc = self.pc.wrapping_add(2);
        self.push_u16(bus, return_pc);
        self.push_byte(bus, self.status_for_stack(true));
        self.set_flag(STATUS_INTERRUPT_DISABLE, true);
        let lo = bus.read(0xFFFE) as u16;
        let hi = bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
        self.tick(bus, 7);
    }

    pub fn return_from_interrupt(&mut self, bus: &mut impl CpuBus) {
        let status = self.pull_byte(bus);
        self.restore_status(status);
        self.pc = self.pull_u16(bus);
        self.tick(bus, 6);
    }

    pub(crate) fn next_pc(&self, byte_len: u8) -> u16 {
        self.pc.wrapping_add(byte_len as u16)
    }

    pub(crate) fn flag(&self, mask: u8) -> bool {
        self.status & mask != 0
    }

    pub(crate) fn set_flag(&mut self, mask: u8, enabled: bool) {
        if enabled {
            self.status |= mask;
        } else {
            self.status &= !mask;
        }
        self.normalize_status();
    }

    pub(crate) fn normalize_status(&mut self) {
        self.status |= STATUS_UNUSED;
        self.status &= !STATUS_BREAK;
    }

    pub(crate) fn set_zero_negative(&mut self, value: u8) {
        self.set_flag(STATUS_ZERO, value == 0);
        self.set_flag(STATUS_NEGATIVE, value & 0x80 != 0);
    }

    pub(crate) fn status_snapshot(&self) -> u8 {
        (self.status | STATUS_UNUSED) & !STATUS_BREAK
    }

    pub(crate) fn status_for_stack(&self, break_flag: bool) -> u8 {
        let mut value = self.status_snapshot();
        if break_flag {
            value |= STATUS_BREAK;
        }
        value
    }

    pub(crate) fn restore_status(&mut self, value: u8) {
        self.status = (value | STATUS_UNUSED) & !STATUS_BREAK;
    }

    pub(crate) fn adc(&mut self, value: u8) {
        let carry_in = u16::from(self.flag(STATUS_CARRY));
        let lhs = self.a;
        let sum = lhs as u16 + value as u16 + carry_in;
        let result = sum as u8;

        self.set_flag(STATUS_CARRY, sum > 0xFF);
        self.set_flag(
            STATUS_OVERFLOW,
            (!(lhs ^ value) & (lhs ^ result) & 0x80) != 0,
        );
        self.a = result;
        self.set_zero_negative(self.a);
    }

    pub(crate) fn sbc(&mut self, value: u8) {
        self.adc(value ^ 0xFF);
    }

    pub(crate) fn compare(&mut self, lhs: u8, rhs: u8) {
        let result = lhs.wrapping_sub(rhs);
        self.set_flag(STATUS_CARRY, lhs >= rhs);
        self.set_zero_negative(result);
    }
}
