mod support;

use std::path::PathBuf;

use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::shell::{
    LoadedRom, PresentationAction, PresentationMode, RuntimeMenuMode, RuntimeSession, ScaleMode,
    apply_presentation_action, compose_runtime_frame, default_presentation_state,
    presentation_action_for_key, window_size_for_presentation,
};
use winit::keyboard::{KeyCode, ModifiersState};

use support::assertions::{assert_framebuffer_eq, assert_framebuffer_ne};

fn mapper0_cartridge() -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    rom.extend(std::iter::repeat_n(0xEA, 0x4000));
    rom.extend(std::iter::repeat_n(0x00, 0x2000));
    load_cartridge_from_bytes(&rom).expect("fixture cartridge should build")
}

fn runtime_session() -> RuntimeSession {
    RuntimeSession::new(
        LoadedRom {
            source_path: PathBuf::from("runtime-view-fixture.nes"),
            mapper_id: 0,
            title: Some(String::from("runtime-view-fixture")),
        },
        mapper0_cartridge(),
    )
}

#[test]
fn compose_runtime_frame_keeps_running_frame_unchanged() {
    let session = runtime_session();
    assert_framebuffer_eq(
        &compose_runtime_frame(&session),
        session.last_presented_frame(),
        "running frame should render unchanged",
    );
}

#[test]
fn compose_runtime_frame_draws_visible_pause_overlay() {
    let mut session = runtime_session();
    let gameplay = *session.last_presented_frame();
    session.open_pause_menu();

    let paused = compose_runtime_frame(&session);

    assert_framebuffer_ne(
        &paused,
        &gameplay,
        "pause overlay should change the composed frame",
    );
    assert!(matches!(
        session.menu_mode(),
        RuntimeMenuMode::PauseMenu { .. }
    ));
}

#[test]
fn compose_runtime_frame_changes_for_remap_and_audio_views() {
    let mut session = runtime_session();
    session.open_pause_menu();
    let pause_frame = compose_runtime_frame(&session);

    session.begin_remap_controls();
    let remap_frame = compose_runtime_frame(&session);
    assert_ne!(remap_frame, pause_frame);

    session.open_pause_menu();
    for _ in 0..4 {
        session
            .handle_runtime_key(winit::keyboard::KeyCode::ArrowDown, true, false)
            .expect("pause navigation should work");
    }
    session
        .handle_runtime_key(winit::keyboard::KeyCode::Enter, true, false)
        .expect("enter should open audio controls");
    let audio_frame = compose_runtime_frame(&session);
    assert_eq!(session.menu_mode(), RuntimeMenuMode::AudioControls);
    assert_framebuffer_ne(
        &audio_frame,
        &pause_frame,
        "audio controls should differ from pause menu",
    );
    assert_framebuffer_ne(
        &audio_frame,
        &remap_frame,
        "audio controls should differ from remap view",
    );
}

#[test]
fn compose_runtime_frame_draws_debug_hud_over_running_frame() {
    let mut session = runtime_session();
    let gameplay = *session.last_presented_frame();

    session
        .handle_runtime_key(winit::keyboard::KeyCode::F1, true, false)
        .expect("F1 should toggle debug hud");

    let debug_frame = compose_runtime_frame(&session);

    assert_framebuffer_ne(
        &debug_frame,
        &gameplay,
        "debug HUD should alter the composed frame",
    );
}

#[test]
fn window_size_for_presentation_snaps_integer_scale() {
    let state = default_presentation_state();
    let size = window_size_for_presentation(state, winit::dpi::PhysicalSize::new(930, 700));

    assert_eq!(size.width, 512);
    assert_eq!(size.height, 480);
}

#[test]
fn window_size_for_presentation_preserves_fit_window_aspect() {
    let mut state = default_presentation_state();
    apply_presentation_action(&mut state, PresentationAction::ToggleScaleMode);
    assert_eq!(state.scale_mode, ScaleMode::FitWindow);

    let size = window_size_for_presentation(state, winit::dpi::PhysicalSize::new(900, 600));

    assert_eq!(size, winit::dpi::PhysicalSize::new(640, 600));
}

#[test]
fn presentation_hotkeys_map_to_expected_actions() {
    assert_eq!(
        presentation_action_for_key(KeyCode::F11, ModifiersState::default(), true, false),
        Some(PresentationAction::ToggleFullscreen)
    );
    assert_eq!(
        presentation_action_for_key(KeyCode::F10, ModifiersState::default(), true, false),
        Some(PresentationAction::ToggleScaleMode)
    );
    assert_eq!(
        presentation_action_for_key(KeyCode::F10, ModifiersState::default(), false, false),
        None
    );

    let mac_fullscreen_modifiers = ModifiersState::SUPER;
    assert_eq!(
        presentation_action_for_key(KeyCode::Enter, mac_fullscreen_modifiers, true, false),
        Some(PresentationAction::ToggleFullscreen)
    );

    assert_eq!(
        presentation_action_for_key(KeyCode::Enter, ModifiersState::ALT, true, false),
        Some(PresentationAction::ToggleFullscreen)
    );

    let mut state = default_presentation_state();
    apply_presentation_action(&mut state, PresentationAction::ToggleFullscreen);
    assert_eq!(state.mode, PresentationMode::FullscreenBorderless);
}
