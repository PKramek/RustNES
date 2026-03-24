use crate::core::bus::CpuBus;

use super::Cpu;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResolvedOperand {
    pub addr: Option<u16>,
    pub value: Option<u8>,
    pub pointer_addr: Option<u16>,
    pub pointer_value: Option<u16>,
    pub branch_target: Option<u16>,
    pub page_crossed: bool,
}

fn read_u16(bus: &mut impl CpuBus, addr: u16) -> u16 {
    let lo = bus.read(addr) as u16;
    let hi = bus.read(addr.wrapping_add(1)) as u16;
    (hi << 8) | lo
}

fn read_u16_zero_page(bus: &mut impl CpuBus, base: u8) -> u16 {
    let lo = bus.read(base as u16) as u16;
    let hi = bus.read(base.wrapping_add(1) as u16) as u16;
    (hi << 8) | lo
}

fn should_defer_absolute_value_read(addr: u16) -> bool {
    matches!(addr, 0x2000..=0x4017)
}

impl Cpu {
    pub(crate) fn resolve_operand(
        &self,
        bus: &mut impl CpuBus,
        mode: AddressingMode,
    ) -> ResolvedOperand {
        match mode {
            AddressingMode::Implied => ResolvedOperand::default(),
            AddressingMode::Accumulator => ResolvedOperand {
                value: Some(self.a),
                ..ResolvedOperand::default()
            },
            AddressingMode::Immediate => {
                let addr = self.pc.wrapping_add(1);
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::ZeroPage => {
                let addr = bus.read(self.pc.wrapping_add(1)) as u16;
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::ZeroPageX => {
                let base = bus.read(self.pc.wrapping_add(1));
                let addr = base.wrapping_add(self.x) as u16;
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::ZeroPageY => {
                let base = bus.read(self.pc.wrapping_add(1));
                let addr = base.wrapping_add(self.y) as u16;
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::Relative => {
                let offset = bus.read(self.pc.wrapping_add(1));
                let next_pc = self.pc.wrapping_add(2);
                let target = next_pc.wrapping_add((offset as i8 as i16) as u16);
                ResolvedOperand {
                    value: Some(offset),
                    branch_target: Some(target),
                    page_crossed: (next_pc & 0xFF00) != (target & 0xFF00),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::Absolute => {
                let addr = read_u16(bus, self.pc.wrapping_add(1));
                ResolvedOperand {
                    addr: Some(addr),
                    value: (!should_defer_absolute_value_read(addr)).then(|| bus.read(addr)),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::AbsoluteX => {
                let base = read_u16(bus, self.pc.wrapping_add(1));
                let addr = base.wrapping_add(self.x as u16);
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    page_crossed: (base & 0xFF00) != (addr & 0xFF00),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::AbsoluteY => {
                let base = read_u16(bus, self.pc.wrapping_add(1));
                let addr = base.wrapping_add(self.y as u16);
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    page_crossed: (base & 0xFF00) != (addr & 0xFF00),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::Indirect => {
                let pointer = read_u16(bus, self.pc.wrapping_add(1));
                let lo = bus.read(pointer) as u16;
                let hi_addr = (pointer & 0xFF00) | (pointer.wrapping_add(1) & 0x00FF);
                let hi = bus.read(hi_addr) as u16;
                let target = (hi << 8) | lo;
                ResolvedOperand {
                    addr: Some(target),
                    pointer_addr: Some(pointer),
                    pointer_value: Some(target),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::IndirectX => {
                let base = bus.read(self.pc.wrapping_add(1));
                let pointer = base.wrapping_add(self.x);
                let target = read_u16_zero_page(bus, pointer);
                ResolvedOperand {
                    addr: Some(target),
                    value: Some(bus.read(target)),
                    pointer_addr: Some(pointer as u16),
                    pointer_value: Some(target),
                    ..ResolvedOperand::default()
                }
            }
            AddressingMode::IndirectY => {
                let pointer = bus.read(self.pc.wrapping_add(1));
                let base = read_u16_zero_page(bus, pointer);
                let addr = base.wrapping_add(self.y as u16);
                ResolvedOperand {
                    addr: Some(addr),
                    value: Some(bus.read(addr)),
                    pointer_addr: Some(pointer as u16),
                    pointer_value: Some(base),
                    branch_target: None,
                    page_crossed: (base & 0xFF00) != (addr & 0xFF00),
                }
            }
        }
    }
}
