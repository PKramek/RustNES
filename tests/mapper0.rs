use RustNES::core::cartridge::{
    Cartridge, CartridgeError, load_cartridge_from_bytes, parse_ines_rom,
};

fn build_ines_rom(prg_banks: u8, chr_banks: u8) -> Vec<u8> {
    let mut bytes = vec![b'N', b'E', b'S', 0x1A, prg_banks, chr_banks, 0, 0];
    bytes.extend_from_slice(&[0; 8]);

    let prg_len = prg_banks as usize * 0x4000;
    let chr_len = chr_banks as usize * 0x2000;

    bytes.extend((0..prg_len).map(|index| (index & 0xFF) as u8));
    bytes.extend((0..chr_len).map(|index| (255 - (index & 0xFF)) as u8));
    bytes
}

fn cartridge_from_bytes(bytes: &[u8]) -> Cartridge {
    load_cartridge_from_bytes(bytes).expect("mapper 0 cartridge should build")
}

#[test]
fn nrom_128_prg_mirrors_upper_bank() {
    let cartridge = cartridge_from_bytes(&build_ines_rom(1, 1));

    assert_eq!(cartridge.cpu_read(0x8000), 0x00);
    assert_eq!(cartridge.cpu_read(0x8001), 0x01);
    assert_eq!(cartridge.cpu_read(0xC000), 0x00);
    assert_eq!(cartridge.cpu_read(0xFFFF), 0xFF);
}

#[test]
fn nrom_256_maps_full_prg_linearly() {
    let cartridge = cartridge_from_bytes(&build_ines_rom(2, 1));

    assert_eq!(cartridge.cpu_read(0x8000), 0x00);
    assert_eq!(cartridge.cpu_read(0xBFFF), 0xFF);
    assert_eq!(cartridge.cpu_read(0xC000), 0x00);
    assert_eq!(cartridge.cpu_read(0xC001), 0x01);
}

#[test]
fn chr_rom_reads_route_through_ppu_window() {
    let cartridge = cartridge_from_bytes(&build_ines_rom(1, 1));

    assert_eq!(cartridge.ppu_read(0x0000), 0xFF);
    assert_eq!(cartridge.ppu_read(0x0001), 0xFE);
    assert_eq!(cartridge.ppu_read(0x1FFF), 0x00);
}

#[test]
fn chr_ram_fallback_allows_ppu_writes() {
    let image = parse_ines_rom(&build_ines_rom(1, 0)).expect("CHR-RAM cartridge should parse");
    let mut cartridge = Cartridge::from_image(image).expect("CHR-RAM cartridge should construct");

    cartridge.ppu_write(0x0010, 0x7A);
    assert_eq!(cartridge.ppu_read(0x0010), 0x7A);
}

#[test]
fn mapper0_exposes_prg_ram_at_6000() {
    let mut cartridge = cartridge_from_bytes(&build_ines_rom(1, 1));

    cartridge.cpu_write(0x6000, 0x12);
    cartridge.cpu_write(0x7FFF, 0x34);

    assert_eq!(cartridge.cpu_read(0x6000), 0x12);
    assert_eq!(cartridge.cpu_read(0x7FFF), 0x34);
}

#[test]
fn unsupported_mapper_remains_strictly_rejected() {
    let mut rom = build_ines_rom(1, 1);
    rom[6] = 0x10;

    let error = load_cartridge_from_bytes(&rom).expect_err("unsupported mapper must fail");
    assert_eq!(
        error,
        CartridgeError::UnsupportedMapper {
            mapper: 1,
            reason: String::from("RustNES v1 supports Mapper 0 / NROM only"),
        }
    );
}
