use crate::core::bus::CpuBus;

use super::{
    AddressingMode, Cpu, CpuError, OperandBusPhase, ResolvedOperand, STATUS_CARRY, STATUS_DECIMAL,
    STATUS_INTERRUPT_DISABLE, STATUS_NEGATIVE, STATUS_OVERFLOW, STATUS_ZERO, StepRecord,
    opcode_meta,
};

impl Cpu {
    pub fn step_instruction(&mut self, bus: &mut impl CpuBus) -> Result<StepRecord, CpuError> {
        self.pending_interrupts = bus.interrupt_lines();

        let opcode = bus.read(self.pc);
        let meta = *opcode_meta(opcode);
        if meta.base_cycles == 0 {
            return Err(CpuError::UnsupportedOpcode {
                opcode,
                pc: self.pc,
            });
        }

        let operand = self.resolve_operand(bus, meta.mode);
        let record = StepRecord {
            pc_before: self.pc,
            opcode,
            bytes: [
                bus.read(self.pc),
                bus.read(self.pc.wrapping_add(1)),
                bus.read(self.pc.wrapping_add(2)),
            ],
            byte_len: meta.bytes,
            a: self.a,
            x: self.x,
            y: self.y,
            status: self.status_snapshot(),
            sp: self.sp,
            cyc_before: self.total_cycles,
            operand_addr: operand.addr,
            operand_value: operand.value,
            pointer_addr: operand.pointer_addr,
            pointer_value: operand.pointer_value,
            branch_target: operand.branch_target,
            page_crossed: operand.page_crossed,
        };

        self.execute_opcode(bus, opcode, meta.mode, meta.base_cycles, operand);
        self.instruction_count += 1;
        Ok(record)
    }

    fn execute_opcode(
        &mut self,
        bus: &mut impl CpuBus,
        opcode: u8,
        mode: AddressingMode,
        base_cycles: u8,
        operand: ResolvedOperand,
    ) {
        match opcode {
            0x00 => self.service_brk(bus),
            0x01 | 0x05 | 0x09 | 0x0D | 0x11 | 0x15 | 0x19 | 0x1D => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "ORA");
                self.a |= value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0x06 | 0x0A | 0x0E | 0x16 | 0x1E => {
                let value = self.shift_left(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x08 => {
                self.push_byte(bus, self.status_for_stack(true));
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x10 => self.branch(bus, !self.flag(STATUS_NEGATIVE), operand, base_cycles),
            0x18 => {
                self.set_flag(STATUS_CARRY, false);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x20 => {
                self.push_u16(bus, self.pc.wrapping_add(2));
                self.pc = operand.addr.expect("JSR requires absolute target");
                self.tick(bus, base_cycles);
            }
            0x21 | 0x25 | 0x29 | 0x2D | 0x31 | 0x35 | 0x39 | 0x3D => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "AND");
                self.a &= value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0x03 | 0x07 | 0x0F | 0x13 | 0x17 | 0x1B | 0x1F => {
                let value = self.shift_left(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.a |= value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x24 | 0x2C => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "BIT");
                self.set_flag(STATUS_ZERO, self.a & value == 0);
                self.set_flag(STATUS_NEGATIVE, value & STATUS_NEGATIVE != 0);
                self.set_flag(STATUS_OVERFLOW, value & STATUS_OVERFLOW != 0);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0x26 | 0x2A | 0x2E | 0x36 | 0x3E => {
                let value = self.rotate_left(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x23 | 0x27 | 0x2F | 0x33 | 0x37 | 0x3B | 0x3F => {
                let value = self.rotate_left(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.a &= value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x28 => {
                let status = self.pull_byte(bus);
                self.restore_status(status);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x30 => self.branch(bus, self.flag(STATUS_NEGATIVE), operand, base_cycles),
            0x38 => {
                self.set_flag(STATUS_CARRY, true);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x40 => self.return_from_interrupt(bus),
            0x41 | 0x45 | 0x49 | 0x4D | 0x51 | 0x55 | 0x59 | 0x5D => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "EOR");
                self.a ^= value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0x43 | 0x47 | 0x4F | 0x53 | 0x57 | 0x5B | 0x5F => {
                let value = self.shift_right(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.a ^= value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x46 | 0x4A | 0x4E | 0x56 | 0x5E => {
                let value = self.shift_right(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x48 => {
                self.push_byte(bus, self.a);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x4C | 0x6C => {
                self.pc = operand.addr.expect("JMP requires target address");
                self.tick(bus, base_cycles);
            }
            0x50 => self.branch(bus, !self.flag(STATUS_OVERFLOW), operand, base_cycles),
            0x58 => {
                self.set_flag(STATUS_INTERRUPT_DISABLE, false);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x60 => {
                self.pc = self.pull_u16(bus).wrapping_add(1);
                self.tick(bus, base_cycles);
            }
            0x61 | 0x65 | 0x69 | 0x6D | 0x71 | 0x75 | 0x79 | 0x7D => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "ADC");
                self.adc(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0x63 | 0x67 | 0x6F | 0x73 | 0x77 | 0x7B | 0x7F => {
                let value = self.rotate_right(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.adc(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x66 | 0x6A | 0x6E | 0x76 | 0x7E => {
                let value = self.rotate_right(mode, operand);
                self.write_operand(bus, mode, operand, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x68 => {
                self.a = self.pull_byte(bus);
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x04 | 0x44 | 0x64 | 0x0C | 0x14 | 0x34 | 0x54 | 0x74 | 0x80 | 0xD4 | 0xF4 | 0x1A
            | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
            }
            0x70 => self.branch(bus, self.flag(STATUS_OVERFLOW), operand, base_cycles),
            0x78 => {
                self.set_flag(STATUS_INTERRUPT_DISABLE, true);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x81 | 0x85 | 0x8D | 0x91 | 0x95 | 0x99 | 0x9D => {
                let pre_ticks = self.write_operand_on_bus_phase(bus, base_cycles, operand, self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0x83 | 0x87 | 0x8F | 0x97 => {
                let pre_ticks =
                    self.write_operand_on_bus_phase(bus, base_cycles, operand, self.a & self.x);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0x84 | 0x8C | 0x94 => {
                let pre_ticks = self.write_operand_on_bus_phase(bus, base_cycles, operand, self.y);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0x86 | 0x8E | 0x96 => {
                let pre_ticks = self.write_operand_on_bus_phase(bus, base_cycles, operand, self.x);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0x88 => {
                self.y = self.y.wrapping_sub(1);
                self.set_zero_negative(self.y);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x8A => {
                self.a = self.x;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x90 => self.branch(bus, !self.flag(STATUS_CARRY), operand, base_cycles),
            0x98 => {
                self.a = self.y;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x9A => {
                self.sp = self.x;
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xA0 | 0xA4 | 0xAC | 0xB4 | 0xBC => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "LDY");
                self.y = value;
                self.set_zero_negative(self.y);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0xA1 | 0xA5 | 0xA9 | 0xAD | 0xB1 | 0xB5 | 0xB9 | 0xBD => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "LDA");
                self.a = value;
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0xA3 | 0xA7 | 0xAF | 0xB3 | 0xB7 | 0xBF => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "LAX");
                self.a = value;
                self.x = value;
                self.set_zero_negative(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0xA2 | 0xA6 | 0xAE | 0xB6 | 0xBE => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "LDX");
                self.x = value;
                self.set_zero_negative(self.x);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0xA8 => {
                self.y = self.a;
                self.set_zero_negative(self.y);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xAA => {
                self.x = self.a;
                self.set_zero_negative(self.x);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xB0 => self.branch(bus, self.flag(STATUS_CARRY), operand, base_cycles),
            0xB8 => {
                self.set_flag(STATUS_OVERFLOW, false);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xBA => {
                self.x = self.sp;
                self.set_zero_negative(self.x);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xC0 | 0xC4 | 0xCC => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "CPY");
                self.compare(self.y, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0xC1 | 0xC5 | 0xC9 | 0xCD | 0xD1 | 0xD5 | 0xD9 | 0xDD => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "CMP");
                self.compare(self.a, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0xC3 | 0xC7 | 0xCF | 0xD3 | 0xD7 | 0xDB | 0xDF => {
                let value = operand
                    .value
                    .expect("DCP requires operand value")
                    .wrapping_sub(1);
                bus.write(operand.addr.expect("DCP requires target address"), value);
                self.compare(self.a, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xC6 | 0xCE | 0xD6 | 0xDE => {
                let value = operand
                    .value
                    .expect("DEC requires operand value")
                    .wrapping_sub(1);
                bus.write(operand.addr.expect("DEC requires target address"), value);
                self.set_zero_negative(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xC8 => {
                self.y = self.y.wrapping_add(1);
                self.set_zero_negative(self.y);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xCA => {
                self.x = self.x.wrapping_sub(1);
                self.set_zero_negative(self.x);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xD0 => self.branch(bus, !self.flag(STATUS_ZERO), operand, base_cycles),
            0xD8 => {
                self.set_flag(STATUS_DECIMAL, false);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xE0 | 0xE4 | 0xEC => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "CPX");
                self.compare(self.x, value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, false, pre_ticks);
            }
            0xE1 | 0xE5 | 0xE9 | 0xED | 0xF1 | 0xF5 | 0xF9 | 0xFD => {
                let (value, pre_ticks) =
                    self.read_operand_on_bus_phase(bus, base_cycles, operand, "SBC");
                self.sbc(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.finish_operand_cycles(bus, base_cycles, operand.page_crossed, pre_ticks);
            }
            0xE3 | 0xE7 | 0xEF | 0xF3 | 0xF7 | 0xFB | 0xFF => {
                let value = operand
                    .value
                    .expect("ISC requires operand value")
                    .wrapping_add(1);
                bus.write(operand.addr.expect("ISC requires target address"), value);
                self.sbc(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xE6 | 0xEE | 0xF6 | 0xFE => {
                let value = operand
                    .value
                    .expect("INC requires operand value")
                    .wrapping_add(1);
                bus.write(operand.addr.expect("INC requires target address"), value);
                self.set_zero_negative(value);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xE8 => {
                self.x = self.x.wrapping_add(1);
                self.set_zero_negative(self.x);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xEB => {
                self.sbc(operand.value.expect("SBC requires operand value"));
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xEA => {
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0xF0 => self.branch(bus, self.flag(STATUS_ZERO), operand, base_cycles),
            0xF8 => {
                self.set_flag(STATUS_DECIMAL, true);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            _ => unreachable!("supported opcode table and execution match must stay aligned"),
        }
    }

    fn tick_with_page_cross(&mut self, bus: &mut impl CpuBus, base_cycles: u8, page_crossed: bool) {
        self.finish_operand_cycles(bus, base_cycles, page_crossed, 0);
    }

    fn branch(
        &mut self,
        bus: &mut impl CpuBus,
        condition: bool,
        operand: ResolvedOperand,
        base_cycles: u8,
    ) {
        let next_pc = self.next_pc(2);
        self.pc = next_pc;
        self.tick(bus, base_cycles);
        if condition {
            self.tick(bus, 1);
            if operand.page_crossed {
                self.tick(bus, 1);
            }
            self.pc = operand.branch_target.expect("branch target should exist");
        }
    }

    fn write_operand(
        &mut self,
        bus: &mut impl CpuBus,
        mode: AddressingMode,
        operand: ResolvedOperand,
        value: u8,
    ) {
        match mode {
            AddressingMode::Accumulator => self.a = value,
            _ => bus.write(
                operand.addr.expect("memory operand requires address"),
                value,
            ),
        }
        self.set_zero_negative(value);
    }

    fn shift_left(&mut self, mode: AddressingMode, operand: ResolvedOperand) -> u8 {
        let value = if mode == AddressingMode::Accumulator {
            self.a
        } else {
            operand.value.expect("ASL requires operand value")
        };
        self.set_flag(STATUS_CARRY, value & 0x80 != 0);
        let result = value << 1;
        self.set_zero_negative(result);
        result
    }

    fn shift_right(&mut self, mode: AddressingMode, operand: ResolvedOperand) -> u8 {
        let value = if mode == AddressingMode::Accumulator {
            self.a
        } else {
            operand.value.expect("LSR requires operand value")
        };
        self.set_flag(STATUS_CARRY, value & 0x01 != 0);
        let result = value >> 1;
        self.set_zero_negative(result);
        result
    }

    fn rotate_left(&mut self, mode: AddressingMode, operand: ResolvedOperand) -> u8 {
        let value = if mode == AddressingMode::Accumulator {
            self.a
        } else {
            operand.value.expect("ROL requires operand value")
        };
        let carry_in = u8::from(self.flag(STATUS_CARRY));
        self.set_flag(STATUS_CARRY, value & 0x80 != 0);
        let result = (value << 1) | carry_in;
        self.set_zero_negative(result);
        result
    }

    fn rotate_right(&mut self, mode: AddressingMode, operand: ResolvedOperand) -> u8 {
        let value = if mode == AddressingMode::Accumulator {
            self.a
        } else {
            operand.value.expect("ROR requires operand value")
        };
        let carry_in = if self.flag(STATUS_CARRY) { 0x80 } else { 0 };
        self.set_flag(STATUS_CARRY, value & 0x01 != 0);
        let result = (value >> 1) | carry_in;
        self.set_zero_negative(result);
        result
    }

    fn read_operand_on_bus_phase(
        &mut self,
        bus: &mut impl CpuBus,
        base_cycles: u8,
        operand: ResolvedOperand,
        name: &'static str,
    ) -> (u8, u8) {
        match (operand.bus_phase, operand.addr, operand.value) {
            (OperandBusPhase::Immediate, _, Some(value)) => (value, 0),
            (OperandBusPhase::Final, Some(addr), _) => {
                let pre_ticks = base_cycles
                    .saturating_add(u8::from(operand.page_crossed))
                    .saturating_sub(3);
                self.tick(bus, pre_ticks);
                (bus.read(addr), pre_ticks)
            }
            (_, _, Some(value)) => (value, 0),
            _ => panic!("{name} requires operand value"),
        }
    }

    fn write_operand_on_bus_phase(
        &mut self,
        bus: &mut impl CpuBus,
        base_cycles: u8,
        operand: ResolvedOperand,
        value: u8,
    ) -> u8 {
        let addr = operand.addr.expect("memory operand requires address");
        match operand.bus_phase {
            OperandBusPhase::Immediate => {
                bus.write(addr, value);
                0
            }
            OperandBusPhase::Final => {
                let pre_ticks = base_cycles
                    .saturating_add(u8::from(operand.page_crossed))
                    .saturating_sub(2);
                self.tick(bus, pre_ticks);
                bus.write(addr, value);
                pre_ticks
            }
        }
    }

    fn finish_operand_cycles(
        &mut self,
        bus: &mut impl CpuBus,
        base_cycles: u8,
        page_crossed: bool,
        pre_ticks: u8,
    ) {
        let total_cycles = base_cycles.saturating_add(u8::from(page_crossed));
        let remaining_cycles = total_cycles.saturating_sub(pre_ticks);
        self.tick(bus, remaining_cycles);
    }
}
