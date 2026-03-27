use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use RustNES::shell::{
    App, AppState, OpenRomOutcome, PauseMenuAction, PauseState, RuntimeActionError,
    RuntimeMenuMode, RuntimeSession,
};
use winit::keyboard::KeyCode;

fn unique_rom_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("rustnes-controls-{name}-{nanos}.nes"))
}

fn build_ines_rom(prg_banks: u8, chr_banks: u8, flags6: u8, flags7: u8) -> Vec<u8> {
    let mut bytes = vec![b'N', b'E', b'S', 0x1A, prg_banks, chr_banks, flags6, flags7];
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend(std::iter::repeat_n(0xAA, prg_banks as usize * 0x4000));
    bytes.extend(std::iter::repeat_n(0xBB, chr_banks as usize * 0x2000));
    bytes
}

fn write_rom_fixture(name: &str, contents: &[u8]) -> PathBuf {
    let path = unique_rom_path(name);
    fs::write(&path, contents).expect("test ROM should write");
    path
}

fn load_runtime_session() -> RuntimeSession {
    let path = write_rom_fixture("runtime-controls", &build_ines_rom(1, 1, 0, 0));
    let mut app = App::new();
    let outcome = app.open_path_with_confirmation(path, |_current, _next| true);
    assert_eq!(outcome, OpenRomOutcome::Loaded);

    match app.into_state() {
        AppState::Loaded(session) => *session,
        state => panic!("expected runtime session, got {state:?}"),
    }
}

#[test]
fn pause_and_resume_control_frame_progression() {
    let mut session = load_runtime_session();
    let frame_before = session.console().bus().ppu().frame();

    session.open_pause_menu();
    assert_eq!(session.pause_state(), PauseState::Paused);
    assert!(matches!(
        session.menu_mode(),
        RuntimeMenuMode::PauseMenu { .. }
    ));
    assert!(
        !session
            .advance_until_next_frame()
            .expect("paused runtime should not step")
    );
    assert_eq!(session.console().bus().ppu().frame(), frame_before);

    session.resume();
    assert_eq!(session.pause_state(), PauseState::Running);
    assert!(
        session
            .advance_until_next_frame()
            .expect("resumed runtime should step")
    );
    assert!(session.console().bus().ppu().frame() > frame_before);
}

#[test]
fn soft_reset_preserves_bindings_and_preferences() {
    let mut session = load_runtime_session();
    session.remap_button(RustNES::shell::NesButton::A, KeyCode::KeyQ);
    session.preferences_mut().master_volume = 0.4;
    session.preferences_mut().muted = true;
    session.open_pause_menu();
    assert_eq!(session.selected_pause_action(), PauseMenuAction::Resume);
    session
        .handle_runtime_key(KeyCode::ArrowDown, true, false)
        .expect("menu navigation should work");
    session
        .handle_runtime_key(KeyCode::Enter, true, false)
        .expect("soft reset should succeed");

    assert_eq!(session.bindings().a, KeyCode::KeyQ);
    assert_eq!(session.preferences().master_volume, 0.4);
    assert!(session.preferences().muted);
}

#[test]
fn reload_current_rom_uses_existing_source_path_and_preserves_preferences() {
    let mut session = load_runtime_session();
    let original_path = session.rom.source_path.clone();
    session.preferences_mut().master_volume = 0.6;
    session.remap_button(RustNES::shell::NesButton::B, KeyCode::KeyC);

    session.reload_current_rom().expect("reload should succeed");

    assert_eq!(session.rom.source_path, original_path);
    assert_eq!(session.preferences().master_volume, 0.6);
    assert_eq!(session.bindings().b, KeyCode::KeyC);
}

#[test]
fn reload_failure_returns_calm_structured_error() {
    let mut session = load_runtime_session();
    let doomed_path = session.rom.source_path.clone();
    fs::remove_file(&doomed_path).expect("fixture ROM should delete");

    let error = session
        .reload_current_rom()
        .expect_err("reload should fail when ROM is gone");
    let message = error.diagnostic_message();

    match error {
        RuntimeActionError::ReloadCurrentRom { .. } => {}
        other => panic!("expected reload error, got {other:?}"),
    }
    assert!(message.contains("could not reload the current ROM"));
    assert!(message.contains("could not read ROM"));
}
