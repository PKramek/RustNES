use super::Mirroring;

pub trait Mapper: Send + Sync {
    fn cpu_read(&self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, value: u8);
    fn ppu_read(&self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, value: u8);
    fn mirroring(&self) -> Mirroring;
}
