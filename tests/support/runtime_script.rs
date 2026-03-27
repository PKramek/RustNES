use std::fs;
use std::path::PathBuf;

use RustNES::shell::{App, AppState, OpenRomOutcome, RuntimeActionError, RuntimeSession};
use winit::keyboard::KeyCode;

use super::{unique_temp_path, write_rom};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputStep {
    pub key: KeyCode,
    pub pressed: bool,
    pub repeat: bool,
    pub frames_after: usize,
}

impl InputStep {
    pub fn press(key: KeyCode, frames_after: usize) -> Self {
        Self {
            key,
            pressed: true,
            repeat: false,
            frames_after,
        }
    }

    pub fn release(key: KeyCode, frames_after: usize) -> Self {
        Self {
            key,
            pressed: false,
            repeat: false,
            frames_after,
        }
    }
}

pub fn build_ines_rom(prg_banks: u8, chr_banks: u8, flags6: u8, flags7: u8) -> Vec<u8> {
    let mut bytes = vec![b'N', b'E', b'S', 0x1A, prg_banks, chr_banks, flags6, flags7];
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend(std::iter::repeat_n(0xAA, prg_banks as usize * 0x4000));
    bytes.extend(std::iter::repeat_n(0xBB, chr_banks as usize * 0x2000));
    bytes
}

pub fn write_rom_fixture(name: &str, contents: &[u8]) -> PathBuf {
    let path = unique_temp_path(name, "nes");
    fs::write(&path, contents).expect("test ROM should write");
    path
}

pub fn load_runtime_session(path: PathBuf) -> RuntimeSession {
    let mut app = App::new();
    let outcome = app.open_path_with_confirmation(path, |_current, _next| true);
    assert_eq!(outcome, OpenRomOutcome::Loaded);

    match app.into_state() {
        AppState::Loaded(session) => *session,
        state => panic!("expected runtime session, got {state:?}"),
    }
}

pub fn load_runtime_session_from_bytes(name: &str, contents: &[u8]) -> RuntimeSession {
    load_runtime_session(write_rom_fixture(name, contents))
}

pub fn write_loop_rom(name: &str) -> PathBuf {
    let path = unique_temp_path(name, "nes");
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

pub fn advance_frames(session: &mut RuntimeSession, frames: usize) {
    for frame_index in 0..frames {
        let advanced = session
            .advance_until_next_frame()
            .unwrap_or_else(|error| panic!("frame {frame_index} should advance: {error}"));
        assert!(advanced, "expected frame {frame_index} to advance");
    }
}

pub fn run_input_script(
    session: &mut RuntimeSession,
    script: &[InputStep],
) -> Result<(), RuntimeActionError> {
    for step in script {
        session.handle_runtime_key(step.key, step.pressed, step.repeat)?;
        advance_frames(session, step.frames_after);
    }
    Ok(())
}
