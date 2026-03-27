use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use RustNES::core::cartridge::CartridgeError;
use RustNES::shell::{
    AppState, BootOptions, Launcher, LoadRomError, OpenRomOutcome, initial_rom_arg,
    load_rom_from_path,
};

fn unique_rom_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("rustnes-{name}-{nanos}.nes"))
}

fn build_ines_rom(prg_banks: u8, chr_banks: u8, flags6: u8, flags7: u8) -> Vec<u8> {
    let mut bytes = vec![b'N', b'E', b'S', 0x1A, prg_banks, chr_banks, flags6, flags7];
    bytes.extend_from_slice(&[0; 8]);
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
    bytes.extend(std::iter::repeat_n(0xAA, spec.prg_banks as usize * 0x4000));
    bytes.extend(std::iter::repeat_n(0xBB, spec.chr_banks as usize * 0x2000));
    bytes
}

fn write_rom_fixture(name: &str, contents: &[u8]) -> PathBuf {
    let path = unique_rom_path(name);
    fs::write(&path, contents).expect("test ROM should write");
    path
}

#[test]
fn successful_path_based_load_returns_metadata_and_cartridge() {
    let path = write_rom_fixture("valid-mapper0", &build_ines_rom(1, 1, 0, 0));

    let (loaded, cartridge) = load_rom_from_path(&path).expect("valid mapper 0 ROM should load");

    assert_eq!(loaded.source_path, path);
    assert_eq!(loaded.mapper_id, 0);
    assert_eq!(cartridge.header().mapper_id, 0);
}

#[test]
fn supported_nes2_mapper0_loads_through_the_shared_rom_path() {
    let path = write_rom_fixture(
        "supported-nes2-mapper0",
        &build_nes2_rom(Nes2RomSpec {
            prg_banks: 2,
            chr_banks: 1,
            flags6: 0,
            mapper_or_submapper: 0,
            size_msb: 0,
            prg_ram_fields: 0,
            chr_ram_fields: 0,
            timing: 2,
            console_type: 0,
            misc_roms: 0,
            expansion_device: 1,
        }),
    );

    let (loaded, cartridge) =
        load_rom_from_path(&path).expect("supported NES 2.0 mapper 0 ROM should load");

    assert_eq!(loaded.source_path, path);
    assert_eq!(loaded.mapper_id, 0);
    assert_eq!(cartridge.header().mapper_id, 0);
}

#[test]
fn parser_diagnostic_propagation_stays_structured() {
    let path = write_rom_fixture("invalid-magic", &[0, 1, 2, 3]);

    let error = load_rom_from_path(&path).expect_err("invalid ROM should fail");

    match error {
        LoadRomError::Cartridge {
            source: CartridgeError::TruncatedRom { expected, actual },
            ..
        } => {
            assert_eq!(expected, 16);
            assert_eq!(actual, 4);
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

#[test]
fn unsupported_mapper_message_is_detailed_and_calm() {
    let path = write_rom_fixture("unsupported-mapper", &build_ines_rom(1, 1, 0x10, 0));

    let error = load_rom_from_path(&path).expect_err("unsupported mapper should fail");
    let message = error.diagnostic_message();

    assert!(message.contains("unsupported mapper 1"));
    assert!(message.contains("Mapper 0 / NROM only"));
}

#[test]
fn unsupported_nes2_scope_keeps_a_calm_loader_message() {
    let path = write_rom_fixture(
        "unsupported-nes2-scope",
        &build_nes2_rom(Nes2RomSpec {
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
        }),
    );

    let error = load_rom_from_path(&path).expect_err("out-of-scope NES 2.0 ROM should fail");
    let message = error.diagnostic_message();

    assert!(message.contains("unsupported ROM format"));
    assert!(message.contains("NES 2.0"));
}

#[test]
fn launcher_returns_to_launcher_after_dismissing_load_error() {
    let path = write_rom_fixture("bad-rom", &[0, 1, 2, 3]);
    let mut launcher = Launcher::boot(BootOptions { initial_rom: None });

    let outcome = launcher.open_path_with_confirmation(path, |_current, _next| true);

    assert_eq!(outcome, OpenRomOutcome::Failed);
    assert!(matches!(launcher.state(), AppState::LoadFailed(_)));

    launcher.dismiss_error();
    assert!(matches!(launcher.state(), AppState::Launcher));
}

#[test]
fn launcher_requires_confirmation_before_replacing_loaded_rom() {
    let first = write_rom_fixture("first-rom", &build_ines_rom(1, 1, 0, 0));
    let second = write_rom_fixture("second-rom", &build_ines_rom(1, 1, 0, 0));
    let mut launcher = Launcher::boot(BootOptions {
        initial_rom: Some(first.clone()),
    });

    assert!(matches!(launcher.state(), AppState::Loaded(_)));

    let outcome = launcher.open_path_with_confirmation(second, |_current, _next| false);

    assert_eq!(outcome, OpenRomOutcome::CancelledReplace);
    match launcher.state() {
        AppState::Loaded(session) => assert_eq!(session.rom.source_path, first),
        state => panic!("expected loaded state after cancelled replace, got {state:?}"),
    }
}

#[test]
fn cli_boot_argument_reuses_the_same_initial_rom_path_parsing() {
    let args = vec![OsString::from("rustnes"), OsString::from("example.nes")];

    let initial =
        initial_rom_arg(args).expect("second positional arg should be treated as initial ROM");
    assert_eq!(initial, PathBuf::from("example.nes"));
}
