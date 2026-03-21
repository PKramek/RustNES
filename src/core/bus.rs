use crate::core::cartridge::Cartridge;
use crate::core::io::{ControllerPort, OamDmaPort};
use crate::core::ppu::Ppu;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InterruptLines {
    pub irq: bool,
    pub nmi: bool,
    pub reset: bool,
}

pub trait CpuBus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn tick(&mut self);
    fn interrupt_lines(&self) -> InterruptLines;
}

#[derive(Debug)]
pub struct Bus {
    cpu_ram: [u8; 0x800],
    cartridge: Cartridge,
    ppu: Ppu,
    controller1: ControllerPort,
    controller2: ControllerPort,
    dma: OamDmaPort,
    interrupt_lines: InterruptLines,
    total_cpu_cycles: u64,
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Self {
            cpu_ram: [0; 0x800],
            cartridge,
            ppu: Ppu::default(),
            controller1: ControllerPort::default(),
            controller2: ControllerPort::default(),
            dma: OamDmaPort::default(),
            interrupt_lines: InterruptLines::default(),
            total_cpu_cycles: 0,
        }
    }

    pub fn normalize_cpu_ram_addr(addr: u16) -> usize {
        (addr as usize) & 0x07FF
    }

    pub fn normalize_ppu_register_addr(addr: u16) -> u16 {
        0x2000 | (addr & 0x0007)
    }

    pub fn cartridge(&self) -> &Cartridge {
        &self.cartridge
    }

    pub fn cartridge_mut(&mut self) -> &mut Cartridge {
        &mut self.cartridge
    }

    pub fn dma(&self) -> &OamDmaPort {
        &self.dma
    }

    pub fn dma_mut(&mut self) -> &mut OamDmaPort {
        &mut self.dma
    }

    pub fn ppu(&self) -> &Ppu {
        &self.ppu
    }

    pub fn ppu_mut(&mut self) -> &mut Ppu {
        &mut self.ppu
    }

    pub fn controller1(&self) -> &ControllerPort {
        &self.controller1
    }

    pub fn controller1_mut(&mut self) -> &mut ControllerPort {
        &mut self.controller1
    }

    pub fn controller2(&self) -> &ControllerPort {
        &self.controller2
    }

    pub fn controller2_mut(&mut self) -> &mut ControllerPort {
        &mut self.controller2
    }

    pub fn set_interrupt_lines(&mut self, interrupt_lines: InterruptLines) {
        self.interrupt_lines = interrupt_lines;
    }

    pub fn total_cpu_cycles(&self) -> u64 {
        self.total_cpu_cycles
    }

    pub fn read_u16(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    pub fn read_u16_bug(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi_addr = (addr & 0xFF00) | addr.wrapping_add(1) & 0x00FF;
        let hi = self.read(hi_addr) as u16;
        (hi << 8) | lo
    }
}

impl CpuBus for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[Self::normalize_cpu_ram_addr(addr)],
            0x2000..=0x3FFF => {
                let normalized = Self::normalize_ppu_register_addr(addr);
                self.ppu.cpu_read_register(normalized, &self.cartridge)
            }
            0x4014 => self.dma.last_page().unwrap_or(0),
            0x4016 => self.controller1.read(),
            0x4017 => self.controller2.read(),
            0x4020..=0xFFFF => self.cartridge.cpu_read(addr),
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[Self::normalize_cpu_ram_addr(addr)] = value,
            0x2000..=0x3FFF => {
                let normalized = Self::normalize_ppu_register_addr(addr);
                let (ppu, cartridge) = (&mut self.ppu, &mut self.cartridge);
                ppu.cpu_write_register(normalized, value, cartridge);
            }
            0x4014 => self.dma.request(value),
            0x4016 => self.controller1.write_strobe(value),
            0x4017 => self.controller2.write_strobe(value),
            0x4020..=0xFFFF => self.cartridge.cpu_write(addr, value),
            _ => {}
        }
    }

    fn tick(&mut self) {
        self.total_cpu_cycles += 1;
        for _ in 0..3 {
            self.ppu.tick();
        }
    }

    fn interrupt_lines(&self) -> InterruptLines {
        InterruptLines {
            irq: self.interrupt_lines.irq,
            reset: self.interrupt_lines.reset,
            nmi: self.interrupt_lines.nmi || self.ppu.nmi_line(),
        }
    }
}