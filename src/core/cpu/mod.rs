use thiserror::Error;

use crate::core::bus::{CpuBus, InterruptLines};

const STATUS_BREAK: u8 = 0b0001_0000;
const STATUS_UNUSED: u8 = 0b0010_0000;
const STATUS_INTERRUPT_DISABLE: u8 = 0b0000_0100;

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
        self.push_byte(bus, (value & 0x00FF) as u8);
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
        self.push_byte(bus, (self.status | STATUS_UNUSED) & !STATUS_BREAK);
        self.status |= STATUS_INTERRUPT_DISABLE;
        let lo = bus.read(0xFFFA) as u16;
        let hi = bus.read(0xFFFB) as u16;
        self.pc = (hi << 8) | lo;
        self.tick(bus, 7);
    }

    pub fn service_irq(&mut self, bus: &mut impl CpuBus) {
        self.push_u16(bus, self.pc);
        self.push_byte(bus, (self.status | STATUS_UNUSED) & !STATUS_BREAK);
        self.status |= STATUS_INTERRUPT_DISABLE;
        let lo = bus.read(0xFFFE) as u16;
        let hi = bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
        self.tick(bus, 7);
    }

    pub fn service_brk(&mut self, bus: &mut impl CpuBus) {
        let return_pc = self.pc.wrapping_add(2);
        self.push_u16(bus, return_pc);
        self.push_byte(bus, self.status | STATUS_BREAK | STATUS_UNUSED);
        self.status |= STATUS_INTERRUPT_DISABLE;
        let lo = bus.read(0xFFFE) as u16;
        let hi = bus.read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
        self.tick(bus, 7);
    }

    pub fn return_from_interrupt(&mut self, bus: &mut impl CpuBus) {
        let status = self.pull_byte(bus);
        self.status = (status | STATUS_UNUSED) & !STATUS_BREAK;
        self.pc = self.pull_u16(bus);
        self.tick(bus, 6);
    }

    pub fn step_instruction(&mut self, bus: &mut impl CpuBus) -> Result<StepRecord, CpuError> {
        self.pending_interrupts = bus.interrupt_lines();

        let record = StepRecord {
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

        match record.opcode {
            0xEA => {
                self.pc = self.pc.wrapping_add(1);
                self.tick(bus, 2);
            }
            0x00 => self.service_brk(bus),
            0x40 => self.return_from_interrupt(bus),
            _ => return Err(CpuError::Unimplemented),
        }

        self.instruction_count += 1;
        Ok(record)
    }
}