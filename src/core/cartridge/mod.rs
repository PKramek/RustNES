mod error;
mod image;
mod ines;
mod mapper;
mod nrom;

use std::path::Path;

pub use error::CartridgeError;
pub use image::{ChrStorage, InesFlags6, InesHeader, InesRom, Mirroring};
pub use ines::parse_ines_rom;
pub use mapper::Mapper;
pub use nrom::Mapper0;

pub struct Cartridge {
    header: InesHeader,
    mapper: Box<dyn Mapper>,
}

impl std::fmt::Debug for Cartridge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Cartridge")
            .field("header", &self.header)
            .field("mirroring", &self.mirroring())
            .finish_non_exhaustive()
    }
}

impl Cartridge {
    pub fn from_image(image: InesRom) -> Result<Self, CartridgeError> {
        if image.header.mapper_id != 0 {
            return Err(CartridgeError::UnsupportedMapper {
                mapper: image.header.mapper_id,
                reason: String::from("RustNES v1 supports Mapper 0 / NROM only"),
            });
        }

        let header = image.header;
        let mapper = Box::new(Mapper0::new(image)?);

        Ok(Self { header, mapper })
    }

    pub fn header(&self) -> &InesHeader {
        &self.header
    }

    pub fn cpu_read(&self, addr: u16) -> u8 {
        self.mapper.cpu_read(addr)
    }

    pub fn cpu_write(&mut self, addr: u16, value: u8) {
        self.mapper.cpu_write(addr, value);
    }

    pub fn ppu_read(&self, addr: u16) -> u8 {
        self.mapper.ppu_read(addr)
    }

    pub fn ppu_write(&mut self, addr: u16, value: u8) {
        self.mapper.ppu_write(addr, value);
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring()
    }

    pub fn mapper(&self) -> &dyn Mapper {
        self.mapper.as_ref()
    }
}

pub fn load_cartridge_from_bytes(bytes: &[u8]) -> Result<Cartridge, CartridgeError> {
    let image = parse_ines_rom(bytes)?;
    Cartridge::from_image(image)
}

pub fn load_cartridge_from_path(path: impl AsRef<Path>) -> Result<Cartridge, CartridgeError> {
    let bytes = std::fs::read(path.as_ref()).map_err(|error| CartridgeError::UnsupportedCartridgeLayout {
        reason: format!("unable to read ROM file: {error}"),
    })?;

    load_cartridge_from_bytes(&bytes)
}