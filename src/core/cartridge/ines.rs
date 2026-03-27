use super::{CartridgeError, ChrStorage, InesFlags6, InesHeader, InesRom, Mirroring};

const INES_HEADER_LEN: usize = 16;
const TRAINER_LEN: usize = 512;
const PRG_BANK_LEN: usize = 0x4000;
const CHR_BANK_LEN: usize = 0x2000;

const NES2_IDENTIFIER_MASK: u8 = 0x0C;
const NES2_IDENTIFIER_VALUE: u8 = 0x08;

pub fn parse_ines_rom(bytes: &[u8]) -> Result<InesRom, CartridgeError> {
    if bytes.len() < INES_HEADER_LEN {
        return Err(CartridgeError::TruncatedRom {
            expected: INES_HEADER_LEN,
            actual: bytes.len(),
        });
    }

    if &bytes[..4] != b"NES\x1A" {
        return Err(CartridgeError::InvalidMagic);
    }

    let flags6 = InesFlags6::from_bits_truncate(bytes[6]);
    let flags7 = bytes[7];
    let is_nes2 = flags7 & NES2_IDENTIFIER_MASK == NES2_IDENTIFIER_VALUE;

    if is_nes2 {
        validate_supported_nes2_subset(bytes)?;
    } else if bytes[12..16].iter().any(|byte| *byte != 0) {
        return Err(CartridgeError::DirtyHeader {
            reason: String::from(
                "header padding bytes 12-15 must be zeroed for strict iNES 1.0 parsing",
            ),
        });
    }

    let prg_rom_banks = parse_nes2_bank_count(bytes[4], bytes[9] & 0x0F, "PRG")?;
    let chr_rom_banks = parse_nes2_bank_count(bytes[5], bytes[9] >> 4, "CHR")?;
    let mapper_id = ((flags7 & 0xF0) as u16) | ((bytes[6] >> 4) as u16);
    let trainer_len = if flags6.contains(InesFlags6::TRAINER_PRESENT) {
        TRAINER_LEN
    } else {
        0
    };
    let prg_len = prg_rom_banks as usize * PRG_BANK_LEN;
    let chr_len = chr_rom_banks as usize * CHR_BANK_LEN;
    let expected_len = INES_HEADER_LEN + trainer_len + prg_len + chr_len;

    if bytes.len() < expected_len {
        return Err(CartridgeError::TruncatedRom {
            expected: expected_len,
            actual: bytes.len(),
        });
    }

    let mirroring = if flags6.contains(InesFlags6::VERTICAL_MIRRORING) {
        Mirroring::Vertical
    } else {
        Mirroring::Horizontal
    };

    let prg_start = INES_HEADER_LEN + trainer_len;
    let prg_end = prg_start + prg_len;
    let chr_end = prg_end + chr_len;

    let prg_rom = bytes[prg_start..prg_end].to_vec().into_boxed_slice();
    let chr = if chr_len == 0 {
        ChrStorage::Ram(Box::new([0; CHR_BANK_LEN]))
    } else {
        ChrStorage::Rom(bytes[prg_end..chr_end].to_vec().into_boxed_slice())
    };

    Ok(InesRom {
        header: InesHeader {
            prg_rom_banks,
            chr_rom_banks,
            flags6,
            flags7,
            mapper_id,
            mirroring,
        },
        prg_rom,
        chr,
    })
}

fn validate_supported_nes2_subset(bytes: &[u8]) -> Result<(), CartridgeError> {
    let mapper_upper_bits = bytes[8] & 0x0F;
    let submapper = bytes[8] >> 4;
    let extended_prg_bits = bytes[9] & 0x0F;
    let extended_chr_bits = bytes[9] >> 4;
    let prg_ram_fields = bytes[10];
    let chr_ram_fields = bytes[11];

    if mapper_upper_bits != 0 {
        return Err(CartridgeError::UnsupportedFormat {
            format: "NES 2.0 extended mapper bits",
        });
    }

    if submapper != 0 {
        return Err(CartridgeError::UnsupportedFormat {
            format: "NES 2.0 submapper",
        });
    }

    if extended_prg_bits != 0 || extended_chr_bits != 0 {
        return Err(CartridgeError::UnsupportedFormat {
            format: "NES 2.0 extended PRG/CHR sizing",
        });
    }

    if prg_ram_fields != 0 || chr_ram_fields != 0 {
        return Err(CartridgeError::UnsupportedFormat {
            format: "NES 2.0 RAM sizing",
        });
    }

    Ok(())
}

fn parse_nes2_bank_count(
    base_count: u8,
    upper_nibble: u8,
    kind: &'static str,
) -> Result<u8, CartridgeError> {
    if upper_nibble == 0 {
        return Ok(base_count);
    }

    Err(CartridgeError::UnsupportedFormat {
        format: match kind {
            "PRG" | "CHR" => "NES 2.0 extended PRG/CHR sizing",
            _ => "NES 2.0",
        },
    })
}
