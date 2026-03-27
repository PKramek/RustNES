use super::{CartridgeError, ChrStorage, InesFlags6, InesRom, Mapper, Mirroring};

const PRG_RAM_SIZE: usize = 0x2000;

pub struct Mapper0 {
    prg_rom: Box<[u8]>,
    prg_ram: Box<[u8; PRG_RAM_SIZE]>,
    chr: ChrStorage,
    mirroring: Mirroring,
}

impl Mapper0 {
    pub fn new(image: InesRom) -> Result<Self, CartridgeError> {
        let prg_len = image.prg_rom.len();
        if prg_len != 0x4000 && prg_len != 0x8000 {
            return Err(CartridgeError::UnsupportedCartridgeLayout {
                reason: format!(
                    "Mapper 0 requires 16 KiB or 32 KiB PRG-ROM, found {prg_len} bytes"
                ),
            });
        }

        if image.header.flags6.contains(InesFlags6::FOUR_SCREEN) {
            return Err(CartridgeError::UnsupportedCartridgeLayout {
                reason: String::from("four-screen nametable layout is out of scope for RustNES v1"),
            });
        }

        match &image.chr {
            ChrStorage::Rom(chr) if chr.len() != 0x2000 => {
                return Err(CartridgeError::UnsupportedCartridgeLayout {
                    reason: format!(
                        "Mapper 0 expects 8 KiB CHR-ROM when CHR-ROM is present, found {} bytes",
                        chr.len()
                    ),
                });
            }
            ChrStorage::Ram(_) | ChrStorage::Rom(_) => {}
        }

        Ok(Self {
            prg_rom: image.prg_rom,
            prg_ram: Box::new([0; PRG_RAM_SIZE]),
            chr: image.chr,
            mirroring: image.header.mirroring,
        })
    }
}

impl Mapper for Mapper0 {
    fn cpu_read(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xFFFF => {
                let offset = (addr - 0x8000) as usize;
                let index = if self.prg_rom.len() == 0x4000 {
                    offset & 0x3FFF
                } else {
                    offset & 0x7FFF
                };
                self.prg_rom[index]
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, value: u8) {
        if let 0x6000..=0x7FFF = addr {
            self.prg_ram[(addr - 0x6000) as usize] = value;
        }
    }

    fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => match &self.chr {
                ChrStorage::Rom(chr) => chr[addr as usize],
                ChrStorage::Ram(chr) => chr[addr as usize],
            },
            _ => 0,
        }
    }

    fn ppu_write(&mut self, addr: u16, value: u8) {
        if let 0x0000..=0x1FFF = addr
            && let ChrStorage::Ram(chr) = &mut self.chr
        {
            chr[addr as usize] = value;
        }
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}
