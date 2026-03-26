mod support;

use RustNES::core::ppu::FRAMEBUFFER_LEN;
use RustNES::shell::{AppState, BootOptions, LoadRomError, PauseState};
use winit::keyboard::KeyCode;

use support::runtime_script::{
    build_ines_rom, load_runtime_session, write_loop_rom, write_rom_fixture,
};
use support::unique_temp_path;

#[test]
fn boot_with_initial_rom_enters_loaded_runtime_session() {
    let rom_path = write_rom_fixture("boot", &build_ines_rom(1, 1, 0, 0));
    let launcher = RustNES::shell::Launcher::boot(BootOptions {
        initial_rom: Some(rom_path.clone()),
    });

    match launcher.state() {
        AppState::Loaded(session) => {
            assert_eq!(session.rom.source_path, rom_path);
            assert_eq!(session.last_presented_frame().len(), FRAMEBUFFER_LEN);
        }
        state => panic!("expected loaded runtime state, got {state:?}"),
    }
}

#[test]
fn runtime_session_preserves_loaded_rom_path_for_future_reload() {
    let rom_path = write_rom_fixture("reload-path", &build_ines_rom(1, 1, 0, 0));
    let session = load_runtime_session(rom_path.clone());

    assert_eq!(session.rom.source_path, rom_path);
}

#[test]
fn paused_session_preserves_last_presented_frame() {
    let rom_path = write_rom_fixture("pause-frame", &build_ines_rom(1, 1, 0, 0));
    let mut session = load_runtime_session(rom_path);
    let before = *session.last_presented_frame();

    session.set_pause_state(RustNES::shell::PauseState::Paused);
    let advanced = session
        .advance_until_next_frame()
        .expect("paused runtime should not error");

    assert!(!advanced);
    assert_eq!(*session.last_presented_frame(), before);
}

#[test]
fn runtime_session_can_step_until_next_frame() {
    let rom_path = write_rom_fixture("frame-step", &build_ines_rom(1, 1, 0, 0));
    let mut session = load_runtime_session(rom_path);
    let start_frame = session.console().bus().ppu().frame();

    let advanced = session
        .advance_until_next_frame()
        .expect("frame stepping should succeed");

    assert!(advanced);
    assert!(session.console().bus().ppu().frame() > start_frame);
}

#[test]
fn runtime_session_keeps_advancing_frames_past_ten_seconds() {
    let rom_path = write_loop_rom("long-run");
    let mut session = load_runtime_session(rom_path.clone());
    let mut last_frame = session.console().bus().ppu().frame();

    for frame_index in 0..900 {
        let advanced = session
            .advance_until_next_frame()
            .expect("runtime should keep producing frames");

        assert!(advanced, "expected runtime to advance frame {frame_index}");
        let current_frame = session.console().bus().ppu().frame();
        assert!(
            current_frame > last_frame,
            "expected frame counter to keep increasing at frame {frame_index}"
        );
        last_frame = current_frame;
    }

    assert_eq!(session.pause_state(), PauseState::Running);

    let _ = std::fs::remove_file(&rom_path);
}

#[test]
fn runtime_bootstrap_error_messages_stay_calm_and_explicit() {
    let error = RustNES::shell::RuntimeBootstrapError::Pixels {
        source: pixels::Error::InvalidTexture(pixels::TextureError::TextureWidth(0)),
    };

    assert!(
        error
            .diagnostic_message()
            .contains("could not start the runtime view")
    );
}

#[test]
fn runtime_debug_snapshot_includes_cpu_ppu_and_trace_details() {
    let rom_path = write_loop_rom("debug-snapshot");
    let mut session = load_runtime_session(rom_path.clone());

    let advanced = session
        .advance_until_next_frame()
        .expect("runtime should produce a frame before dumping debug state");
    assert!(advanced);

    let snapshot = session.debug_snapshot_text();
    assert!(snapshot.contains("RUNTIME DEBUG SNAPSHOT"));
    assert!(snapshot.contains("CPU: PC="));
    assert!(snapshot.contains("PPU: frame="));
    assert!(snapshot.contains("RECENT TRACE:"));

    let _ = std::fs::remove_file(&rom_path);
}

#[test]
fn runtime_debug_hotkey_toggles_overlay() {
    let rom_path = write_loop_rom("debug-hud-toggle");
    let mut session = load_runtime_session(rom_path.clone());

    assert!(!session.debug_overlay_visible());
    session
        .handle_runtime_key(KeyCode::F1, true, false)
        .expect("F1 should toggle runtime debug HUD");
    assert!(session.debug_overlay_visible());
    session
        .handle_runtime_key(KeyCode::F1, true, false)
        .expect("F1 should toggle runtime debug HUD off");
    assert!(!session.debug_overlay_visible());

    let _ = std::fs::remove_file(&rom_path);
}

#[test]
fn load_failures_remain_structured_for_bootstrap_paths() {
    let missing = unique_temp_path("missing", "nes");
    let error = RustNES::shell::load_rom_from_path(&missing).expect_err("missing ROM should fail");

    match error {
        LoadRomError::Io { path, .. } => assert_eq!(path, missing),
        other => panic!("expected IO load error, got {other:?}"),
    }
}
