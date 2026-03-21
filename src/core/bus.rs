use crate::core::cartridge::Cartridge;
use crate::core::io::{ControllerPort, OamDmaPort, PpuPortsStub};

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
    ppu_ports: PpuPortsStub,
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
            ppu_ports: PpuPortsStub::default(),
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
}

impl CpuBus for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[Self::normalize_cpu_ram_addr(addr)],
            0x2000..=0x3FFF => self.ppu_ports.read(Self::normalize_ppu_register_addr(addr)),
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
            0x2000..=0x3FFF => self.ppu_ports.write(Self::normalize_ppu_register_addr(addr), value),
            0x4014 => self.dma.request(value),
            0x4016 => self.controller1.write_strobe(value),
            0x4017 => self.controller2.write_strobe(value),
            0x4020..=0xFFFF => self.cartridge.cpu_write(addr, value),
            _ => {}
        }
    }

    fn tick(&mut self) {
        self.total_cpu_cycles += 1;
    }

    fn interrupt_lines(&self) -> InterruptLines {
        self.interrupt_lines
    }
}