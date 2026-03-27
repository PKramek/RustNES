mod support;

use RustNES::core::ppu::FRAMEBUFFER_LEN;
use RustNES::shell::{AppState, BootOptions, Launcher, OpenRomOutcome};

use support::runtime_script::{build_ines_rom, write_rom_fixture};
use support::unique_temp_path;

#[test]
fn launcher_boot_with_missing_initial_rom_surfaces_recoverable_failure() {
    let missing = unique_temp_path("desktop-missing", "nes");
    let launcher = Launcher::boot(BootOptions {
        initial_rom: Some(missing.clone()),
    });

    match launcher.state() {
        AppState::LoadFailed(failure) => {
            assert!(failure.message.contains("could not read ROM"));
            assert!(failure.message.contains(&missing.display().to_string()));
        }
        state => panic!("expected recoverable load failure, got {state:?}"),
    }
}

#[test]
fn launcher_boot_with_generated_rom_reaches_runtime_surface() {
    let rom_path = write_rom_fixture("desktop-boot", &build_ines_rom(1, 1, 0, 0));
    let launcher = Launcher::boot(BootOptions {
        initial_rom: Some(rom_path.clone()),
    });

    match launcher.state() {
        AppState::Loaded(session) => {
            assert_eq!(session.rom.source_path, rom_path);
            assert_eq!(session.last_presented_frame().len(), FRAMEBUFFER_LEN);
        }
        state => panic!("expected loaded runtime surface, got {state:?}"),
    }
}

#[test]
fn launcher_can_recover_from_failed_boot_by_loading_generated_rom() {
    let missing = unique_temp_path("desktop-recover-missing", "nes");
    let rom_path = write_rom_fixture("desktop-recover", &build_ines_rom(1, 1, 0, 0));
    let mut launcher = Launcher::boot(BootOptions {
        initial_rom: Some(missing),
    });

    assert!(matches!(launcher.state(), AppState::LoadFailed(_)));

    let outcome = launcher.open_path_with_confirmation(rom_path.clone(), |_current, _next| true);
    assert_eq!(outcome, OpenRomOutcome::Loaded);

    match launcher.state() {
        AppState::Loaded(session) => assert_eq!(session.rom.source_path, rom_path),
        state => panic!("expected recovered loaded state, got {state:?}"),
    }
}

#[test]
#[ignore = "Phase 5 presentation toggles are not implemented yet"]
fn presentation_toggle_contract_stays_reserved_for_phase_five() {
    panic!("enable once Phase 5 presentation toggles land");
}
