#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use RustNES::core::cartridge::{Cartridge, load_cartridge_from_bytes};
use RustNES::core::console::Console;

pub fn rom_bytes(program: &[(u16, u8)], reset: u16, nmi: u16, irq: u16) -> Vec<u8> {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 2, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; 0x8000];
    for (addr, value) in program {
        assert!(
            (*addr as usize) >= 0x8000,
            "program bytes must live in cartridge space"
        );
        prg[*addr as usize - 0x8000] = *value;
    }

    prg[0x7FFA] = (nmi & 0x00FF) as u8;
    prg[0x7FFB] = (nmi >> 8) as u8;
    prg[0x7FFC] = (reset & 0x00FF) as u8;
    prg[0x7FFD] = (reset >> 8) as u8;
    prg[0x7FFE] = (irq & 0x00FF) as u8;
    prg[0x7FFF] = (irq >> 8) as u8;

    rom.extend(prg);
    rom.extend(std::iter::repeat_n(0, 0x2000));
    rom
}

pub fn cartridge_from_program(program: &[(u16, u8)], reset: u16, nmi: u16, irq: u16) -> Cartridge {
    load_cartridge_from_bytes(&rom_bytes(program, reset, nmi, irq))
        .expect("fixture cartridge should build")
}

pub fn console_from_program(program: &[(u16, u8)], reset: u16) -> Console {
    let mut console = Console::new(cartridge_from_program(program, reset, reset, reset));
    console.reset();
    console
}

pub fn write_rom(path: impl AsRef<Path>, program: &[(u16, u8)], reset: u16) -> PathBuf {
    let path = path.as_ref().to_path_buf();
    std::fs::write(&path, rom_bytes(program, reset, reset, reset))
        .expect("fixture ROM should write");
    path
}

pub fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("rustnes-{label}-{nanos}.{extension}"))
}
