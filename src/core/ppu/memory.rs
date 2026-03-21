use crate::core::cartridge::{Cartridge, Mirroring};

const NAMETABLE_RAM_SIZE: usize = 0x800;
const PALETTE_RAM_SIZE: usize = 0x20;
const OAM_SIZE: usize = 0x100;

#[derive(Debug)]
pub struct PpuMemory {
    nametable_ram: [u8; NAMETABLE_RAM_SIZE],
    palette_ram: [u8; PALETTE_RAM_SIZE],
    oam: [u8; OAM_SIZE],
}

impl Default for PpuMemory {
    fn default() -> Self {
        Self {
            nametable_ram: [0; NAMETABLE_RAM_SIZE],
            palette_ram: [0; PALETTE_RAM_SIZE],
            oam: [0; OAM_SIZE],
        }
    }
}

impl PpuMemory {
    pub fn read(&self, addr: u16, cartridge: &Cartridge) -> u8 {
        match addr & 0x3FFF {
            0x0000..=0x1FFF => cartridge.ppu_read(addr),
            0x2000..=0x3EFF => {
                self.nametable_ram[self.nametable_index(addr, cartridge.mirroring())]
            }
            0x3F00..=0x3FFF => self.palette_ram[palette_index(addr)],
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8, cartridge: &mut Cartridge) {
        match addr & 0x3FFF {
            0x0000..=0x1FFF => cartridge.ppu_write(addr, value),
            0x2000..=0x3EFF => {
                let index = self.nametable_index(addr, cartridge.mirroring());
                self.nametable_ram[index] = value;
            }
            0x3F00..=0x3FFF => {
                self.palette_ram[palette_index(addr)] = value;
            }
            _ => {}
        }
    }

    pub fn peek(&self, addr: u16, cartridge: &Cartridge) -> u8 {
        self.read(addr, cartridge)
    }

    pub fn oam_read(&self, addr: u8) -> u8 {
        self.oam[addr as usize]
    }

    pub fn oam_write(&mut self, addr: u8, value: u8) {
        self.oam[addr as usize] = value;
    }

    fn nametable_index(&self, addr: u16, mirroring: Mirroring) -> usize {
        let normalized = ((addr & 0x0FFF) % 0x1000) as usize;
        let table = normalized / 0x400;
        let offset = normalized % 0x400;
        let physical_table = match mirroring {
            Mirroring::Vertical => table & 0x01,
            Mirroring::Horizontal => table >> 1,
        };
        physical_table * 0x400 + offset
    }
}

fn palette_index(addr: u16) -> usize {
    let mut index = ((addr - 0x3F00) & 0x001F) as usize;
    if matches!(index, 0x10 | 0x14 | 0x18 | 0x1C) {
        index -= 0x10;
    }
    index
}
