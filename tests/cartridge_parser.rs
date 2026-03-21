use RustNES::core::cartridge::{
    CartridgeError, ChrStorage, Mirroring, load_cartridge_from_bytes, parse_ines_rom,
};

fn build_ines_rom(prg_banks: u8, chr_banks: u8, flags6: u8, flags7: u8) -> Vec<u8> {
    let mut bytes = vec![b'N', b'E', b'S', 0x1A, prg_banks, chr_banks, flags6, flags7];
    bytes.extend_from_slice(&[0; 8]);

    if flags6 & 0b0000_0100 != 0 {
        bytes.extend(std::iter::repeat_n(0x55, 512));
    }

    bytes.extend(std::iter::repeat_n(0xAA, prg_banks as usize * 0x4000));
    bytes.extend(std::iter::repeat_n(0xBB, chr_banks as usize * 0x2000));
    bytes
}

#[test]
fn parses_valid_ines_mapper0_rom() {
    let rom = build_ines_rom(2, 1, 0b0000_0001, 0);
    let parsed = parse_ines_rom(&rom).expect("valid mapper 0 ROM should parse");

    assert_eq!(parsed.header.prg_rom_banks, 2);
    assert_eq!(parsed.header.chr_rom_banks, 1);
    assert_eq!(parsed.header.mapper_id, 0);
    assert_eq!(parsed.header.mirroring, Mirroring::Vertical);
    assert_eq!(parsed.prg_rom.len(), 0x8000);
    match parsed.chr {
        ChrStorage::Rom(chr) => assert_eq!(chr.len(), 0x2000),
        ChrStorage::Ram(_) => panic!("expected CHR-ROM storage"),
    }
}

#[test]
fn trainer_present_rom_skips_exactly_512_bytes() {
    let mut rom = build_ines_rom(1, 1, 0b0000_0100, 0);
    let prg_start = 16 + 512;
    rom[prg_start] = 0xDE;
    rom[prg_start + 1] = 0xAD;

    let parsed = parse_ines_rom(&rom).expect("trainer-backed ROM should parse");
    assert_eq!(&parsed.prg_rom[..2], &[0xDE, 0xAD]);
}

#[test]
fn invalid_magic_maps_to_typed_error_variant() {
    let rom = vec![0, 1, 2, 3];
    let error = parse_ines_rom(&rom).expect_err("invalid magic must fail");

    assert_eq!(
        error,
        CartridgeError::TruncatedRom {
            expected: 16,
            actual: 4
        }
    );
}

#[test]
fn nes2_header_is_rejected() {
    let rom = build_ines_rom(1, 1, 0, 0b0000_1000);
    let error = parse_ines_rom(&rom).expect_err("NES 2.0 should be out of scope");

    assert_eq!(
        error,
        CartridgeError::UnsupportedFormat { format: "NES 2.0" }
    );
}

#[test]
fn truncated_payload_is_rejected() {
    let mut rom = build_ines_rom(2, 1, 0, 0);
    rom.truncate(32);
    let error = parse_ines_rom(&rom).expect_err("truncated payload must fail");

    assert_eq!(
        error,
        CartridgeError::TruncatedRom {
            expected: 16 + (2 * 0x4000) + 0x2000,
            actual: rom.len(),
        }
    );
}

#[test]
fn dirty_header_is_rejected() {
    let mut rom = build_ines_rom(1, 1, 0, 0);
    rom[12] = 1;
    let error = parse_ines_rom(&rom).expect_err("dirty header must fail");

    assert_eq!(
        error,
        CartridgeError::DirtyHeader {
            reason: String::from(
                "header padding bytes 12-15 must be zeroed for strict iNES 1.0 parsing"
            ),
        }
    );
}

#[test]
fn unsupported_mapper_is_rejected_after_parse() {
    let rom = build_ines_rom(1, 1, 0x10, 0);
    let error = load_cartridge_from_bytes(&rom).expect_err("unsupported mapper must fail");

    assert_eq!(
        error,
        CartridgeError::UnsupportedMapper {
            mapper: 1,
            reason: String::from("RustNES v1 supports Mapper 0 / NROM only"),
        }
    );
}

#[test]
fn zero_chr_rom_allocates_chr_ram() {
    let rom = build_ines_rom(1, 0, 0, 0);
    let parsed = parse_ines_rom(&rom).expect("CHR-RAM fallback should parse");

    match parsed.chr {
        ChrStorage::Ram(chr) => assert_eq!(chr.len(), 0x2000),
        ChrStorage::Rom(_) => panic!("expected CHR-RAM allocation when CHR size is zero"),
    }
}
