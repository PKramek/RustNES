mod support;

use RustNES::shell::load_rom_from_path;

use support::runtime_script::{advance_frames, load_runtime_session};
use support::save_state::SaveStateFixture;

#[test]
fn save_state_fixture_uses_generated_rom_and_temp_slot_paths() {
    let fixture = SaveStateFixture::from_program(
        "save-state-fixture",
        &[
            (0xC000, 0xEA),
            (0xC001, 0x4C),
            (0xC002, 0x00),
            (0xC003, 0xC0),
        ],
        0xC000,
    );

    let (loaded, _) = load_rom_from_path(&fixture.rom_path).expect("generated ROM should load");

    assert_eq!(loaded.source_path, fixture.rom_path);
    assert!(!fixture.slot_exists());

    fixture.write_slot(&[0x52, 0x4E, 0x45, 0x53]);
    assert!(fixture.slot_exists());
    assert_eq!(fixture.read_slot(), vec![0x52, 0x4E, 0x45, 0x53]);
}

#[test]
fn save_state_fixture_supports_runtime_progress_before_future_round_trip() {
    let fixture = SaveStateFixture::from_program(
        "save-state-runtime",
        &[
            (0xC000, 0xEA),
            (0xC001, 0x4C),
            (0xC002, 0x00),
            (0xC003, 0xC0),
        ],
        0xC000,
    );
    let mut session = load_runtime_session(fixture.rom_path.clone());
    let start_frame = session.console().bus().ppu().frame();

    advance_frames(&mut session, 3);

    assert!(session.console().bus().ppu().frame() > start_frame);
}

#[test]
#[ignore = "Phase 6 quick save/load surfaces are not implemented yet"]
fn quick_save_load_round_trip_preserves_single_slot_state() {
    panic!("enable once Phase 6 save-state APIs land");
}
