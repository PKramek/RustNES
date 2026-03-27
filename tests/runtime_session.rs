mod support;

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use RustNES::core::ppu::FRAMEBUFFER_LEN;
use RustNES::shell::{App, AppState, BootOptions, LoadRomError, PauseState, RuntimeSession};

use support::write_rom;

fn unique_rom_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("rustnes-runtime-{name}-{nanos}.nes"))
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

fn load_runtime_session(path: PathBuf) -> RuntimeSession {
    let mut app = App::new();
    let outcome = app.open_path_with_confirmation(path, |_current, _next| true);
    assert!(matches!(outcome, RustNES::shell::OpenRomOutcome::Loaded));

    match app.into_state() {
        AppState::Loaded(session) => *session,
        state => panic!("expected runtime session, got {state:?}"),
    }
}

fn write_loop_rom(name: &str) -> PathBuf {
    let path = unique_rom_path(name);
    write_rom(
        &path,
        &[
            (0xC000, 0xEA),
            (0xC001, 0x4C),
            (0xC002, 0x00),
            (0xC003, 0xC0),
        ],
        0xC000,
    );
    path
}

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
fn load_failures_remain_structured_for_bootstrap_paths() {
    let missing = unique_rom_path("missing");
    let error = RustNES::shell::load_rom_from_path(&missing).expect_err("missing ROM should fail");

    match error {
        LoadRomError::Io { path, .. } => assert_eq!(path, missing),
        other => panic!("expected IO load error, got {other:?}"),
    }
}
