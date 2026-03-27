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

struct Nes2RomSpec {
    prg_banks: u8,
    chr_banks: u8,
    flags6: u8,
    mapper_or_submapper: u8,
    size_msb: u8,
    prg_ram_fields: u8,
    chr_ram_fields: u8,
    timing: u8,
    console_type: u8,
    misc_roms: u8,
    expansion_device: u8,
}

fn build_nes2_rom(spec: Nes2RomSpec) -> Vec<u8> {
    let mut bytes = vec![
        b'N',
        b'E',
        b'S',
        0x1A,
        spec.prg_banks,
        spec.chr_banks,
        spec.flags6,
        0b0000_1000,
        spec.mapper_or_submapper,
        spec.size_msb,
        spec.prg_ram_fields,
        spec.chr_ram_fields,
        spec.timing,
        spec.console_type,
        spec.misc_roms,
        spec.expansion_device,
    ];

    if spec.flags6 & 0b0000_0100 != 0 {
        bytes.extend(std::iter::repeat_n(0x55, 512));
    }

    bytes.extend(std::iter::repeat_n(0xAA, spec.prg_banks as usize * 0x4000));
    bytes.extend(std::iter::repeat_n(0xBB, spec.chr_banks as usize * 0x2000));
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
fn nes2_mapper0_subset_is_accepted_when_sizes_stay_in_ines_range() {
    let rom = build_nes2_rom(Nes2RomSpec {
        prg_banks: 2,
        chr_banks: 1,
        flags6: 0b0000_0001,
        mapper_or_submapper: 0,
        size_msb: 0,
        prg_ram_fields: 0,
        chr_ram_fields: 0,
        timing: 2,
        console_type: 0,
        misc_roms: 0,
        expansion_device: 1,
    });
    let parsed = parse_ines_rom(&rom).expect("SMB-style NES 2.0 mapper 0 ROM should parse");

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
fn nes2_headers_that_expand_mapper_scope_stay_rejected() {
    let rom = build_nes2_rom(Nes2RomSpec {
        prg_banks: 1,
        chr_banks: 1,
        flags6: 0,
        mapper_or_submapper: 0x10,
        size_msb: 0,
        prg_ram_fields: 0,
        chr_ram_fields: 0,
        timing: 0,
        console_type: 0,
        misc_roms: 0,
        expansion_device: 0,
    });
    let error = parse_ines_rom(&rom).expect_err("NES 2.0 submappers should stay out of scope");

    assert_eq!(
        error,
        CartridgeError::UnsupportedFormat {
            format: "NES 2.0 submapper"
        }
    );
}

#[test]
fn nes2_headers_that_require_extended_sizing_stay_rejected() {
    let rom = build_nes2_rom(Nes2RomSpec {
        prg_banks: 1,
        chr_banks: 1,
        flags6: 0,
        mapper_or_submapper: 0,
        size_msb: 0x01,
        prg_ram_fields: 0,
        chr_ram_fields: 0,
        timing: 0,
        console_type: 0,
        misc_roms: 0,
        expansion_device: 0,
    });
    let error = parse_ines_rom(&rom).expect_err("NES 2.0 extended sizing should stay out of scope");

    assert_eq!(
        error,
        CartridgeError::UnsupportedFormat {
            format: "NES 2.0 extended PRG/CHR sizing"
        }
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
