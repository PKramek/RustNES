bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct InesFlags6: u8 {
        const VERTICAL_MIRRORING = 0b0000_0001;
        const TRAINER_PRESENT = 0b0000_0100;
        const FOUR_SCREEN = 0b0000_1000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InesHeader {
    pub prg_rom_banks: u8,
    pub chr_rom_banks: u8,
    pub flags6: InesFlags6,
    pub flags7: u8,
    pub mapper_id: u16,
    pub mirroring: Mirroring,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChrStorage {
    Rom(Box<[u8]>),
    Ram(Box<[u8; 0x2000]>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InesRom {
    pub header: InesHeader,
    pub prg_rom: Box<[u8]>,
    pub chr: ChrStorage,
}
