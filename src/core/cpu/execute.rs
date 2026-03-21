use crate::core::bus::CpuBus;

use super::{
    AddressingMode, Cpu, CpuError, ResolvedOperand, STATUS_CARRY, STATUS_DECIMAL,
    STATUS_INTERRUPT_DISABLE, STATUS_NEGATIVE, STATUS_OVERFLOW, STATUS_ZERO, StepRecord,
    opcode_meta,
};

impl Cpu {
    pub fn step_instruction(&mut self, bus: &mut impl CpuBus) -> Result<StepRecord, CpuError> {
        self.pending_interrupts = bus.interrupt_lines();

        let opcode = bus.read(self.pc);
        let meta = *opcode_meta(opcode);
        if !meta.official {
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
                self.a |= operand.value.expect("ORA requires operand value");
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
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
                self.a &= operand.value.expect("AND requires operand value");
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
            }
            0x24 | 0x2C => {
                let value = operand.value.expect("BIT requires operand value");
                self.set_flag(STATUS_ZERO, self.a & value == 0);
                self.set_flag(STATUS_NEGATIVE, value & STATUS_NEGATIVE != 0);
                self.set_flag(STATUS_OVERFLOW, value & STATUS_OVERFLOW != 0);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x26 | 0x2A | 0x2E | 0x36 | 0x3E => {
                let value = self.rotate_left(mode, operand);
                self.write_operand(bus, mode, operand, value);
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
                self.a ^= operand.value.expect("EOR requires operand value");
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
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
                self.adc(operand.value.expect("ADC requires operand value"));
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
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
            0x70 => self.branch(bus, self.flag(STATUS_OVERFLOW), operand, base_cycles),
            0x78 => {
                self.set_flag(STATUS_INTERRUPT_DISABLE, true);
                self.pc = self.next_pc(1);
                self.tick(bus, base_cycles);
            }
            0x81 | 0x85 | 0x8D | 0x91 | 0x95 | 0x99 | 0x9D => {
                bus.write(operand.addr.expect("STA requires target address"), self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x84 | 0x8C | 0x94 => {
                bus.write(operand.addr.expect("STY requires target address"), self.y);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0x86 | 0x8E | 0x96 => {
                bus.write(operand.addr.expect("STX requires target address"), self.x);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
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
                self.y = operand.value.expect("LDY requires operand value");
                self.set_zero_negative(self.y);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
            }
            0xA1 | 0xA5 | 0xA9 | 0xAD | 0xB1 | 0xB5 | 0xB9 | 0xBD => {
                self.a = operand.value.expect("LDA requires operand value");
                self.set_zero_negative(self.a);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
            }
            0xA2 | 0xA6 | 0xAE | 0xB6 | 0xBE => {
                self.x = operand.value.expect("LDX requires operand value");
                self.set_zero_negative(self.x);
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
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
                self.compare(self.y, operand.value.expect("CPY requires operand value"));
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xC1 | 0xC5 | 0xC9 | 0xCD | 0xD1 | 0xD5 | 0xD9 | 0xDD => {
                self.compare(self.a, operand.value.expect("CMP requires operand value"));
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
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
                self.compare(self.x, operand.value.expect("CPX requires operand value"));
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick(bus, base_cycles);
            }
            0xE1 | 0xE5 | 0xE9 | 0xED | 0xF1 | 0xF5 | 0xF9 | 0xFD => {
                self.sbc(operand.value.expect("SBC requires operand value"));
                self.pc = self.next_pc(opcode_meta(opcode).bytes);
                self.tick_with_page_cross(bus, base_cycles, operand.page_crossed);
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
            _ => unreachable!("official opcode table and execution match must stay aligned"),
        }
    }

    fn tick_with_page_cross(&mut self, bus: &mut impl CpuBus, base_cycles: u8, page_crossed: bool) {
        self.tick(bus, base_cycles);
        if page_crossed {
            self.tick(bus, 1);
        }
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
}
