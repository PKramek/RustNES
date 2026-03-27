mod support;

use std::fs;

use RustNES::shell::{
    PauseMenuAction, PauseState, RuntimeActionError, RuntimeMenuMode, RuntimeSession,
};
use winit::keyboard::KeyCode;

use support::runtime_script::{build_ines_rom, load_runtime_session_from_bytes};

fn load_runtime_session() -> RuntimeSession {
    load_runtime_session_from_bytes("runtime-controls", &build_ines_rom(1, 1, 0, 0))
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

#[test]
fn escape_pause_menu_navigation_selects_runtime_actions() {
    let mut session = load_runtime_session();

    session
        .handle_runtime_key(KeyCode::Escape, true, false)
        .expect("escape should open pause menu");
    assert_eq!(session.pause_state(), PauseState::Paused);
    assert_eq!(session.selected_pause_action(), PauseMenuAction::Resume);

    session
        .handle_runtime_key(KeyCode::ArrowDown, true, false)
        .expect("arrow down should select soft reset");
    assert_eq!(session.selected_pause_action(), PauseMenuAction::SoftReset);

    session
        .handle_runtime_key(KeyCode::ArrowDown, true, false)
        .expect("arrow down should select reload");
    assert_eq!(
        session.selected_pause_action(),
        PauseMenuAction::ReloadCurrentRom
    );

    session
        .handle_runtime_key(KeyCode::ArrowDown, true, false)
        .expect("arrow down should select remap controls");
    session
        .handle_runtime_key(KeyCode::Enter, true, false)
        .expect("enter should open remap controls");
    assert!(matches!(
        session.menu_mode(),
        RuntimeMenuMode::RemapControls { .. }
    ));

    session
        .handle_runtime_key(KeyCode::Escape, true, false)
        .expect("escape should return to pause menu");
    assert_eq!(
        session.selected_pause_action(),
        PauseMenuAction::RemapControls
    );

    session
        .handle_runtime_key(KeyCode::ArrowDown, true, false)
        .expect("arrow down should select audio controls");
    assert_eq!(
        session.selected_pause_action(),
        PauseMenuAction::AudioControls
    );
}
