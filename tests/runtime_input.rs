use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use RustNES::shell::{App, AppState, NesButton, OpenRomOutcome, RuntimeSession};
use winit::keyboard::KeyCode;

fn unique_rom_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    std::env::temp_dir().join(format!("rustnes-input-{name}-{nanos}.nes"))
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
    let path = write_rom_fixture("runtime-input", &build_ines_rom(1, 1, 0, 0));
    let mut app = App::new();
    let outcome = app.open_path_with_confirmation(path, |_current, _next| true);
    assert_eq!(outcome, OpenRomOutcome::Loaded);

    match app.into_state() {
        AppState::Loaded(session) => *session,
        state => panic!("expected runtime session, got {state:?}"),
    }
}

#[test]
fn default_bindings_match_phase_four_layout() {
    let session = load_runtime_session();
    let bindings = session.bindings();

    assert_eq!(bindings.b, KeyCode::KeyZ);
    assert_eq!(bindings.a, KeyCode::KeyX);
    assert_eq!(bindings.select, KeyCode::ShiftRight);
    assert_eq!(bindings.start, KeyCode::Enter);
    assert_eq!(bindings.up, KeyCode::ArrowUp);
    assert_eq!(bindings.down, KeyCode::ArrowDown);
    assert_eq!(bindings.left, KeyCode::ArrowLeft);
    assert_eq!(bindings.right, KeyCode::ArrowRight);
}

#[test]
fn remapped_button_applies_immediately_without_restart() {
    let mut session = load_runtime_session();

    session.remap_button(NesButton::A, KeyCode::KeyQ);
    session
        .handle_runtime_key(KeyCode::KeyQ, true, false)
        .expect("remapped key should apply");

    assert_eq!(
        session.resolved_button_mask() & NesButton::A.mask(),
        NesButton::A.mask()
    );
}

#[test]
fn most_recent_direction_wins_on_each_axis() {
    let mut session = load_runtime_session();

    session
        .handle_runtime_key(KeyCode::ArrowLeft, true, false)
        .expect("left press should apply");
    assert_eq!(
        session.resolved_button_mask() & NesButton::Left.mask(),
        NesButton::Left.mask()
    );

    session
        .handle_runtime_key(KeyCode::ArrowRight, true, false)
        .expect("right press should override left");
    assert_eq!(session.resolved_button_mask() & NesButton::Left.mask(), 0);
    assert_eq!(
        session.resolved_button_mask() & NesButton::Right.mask(),
        NesButton::Right.mask()
    );

    session
        .handle_runtime_key(KeyCode::ArrowRight, false, false)
        .expect("releasing right should restore left");
    assert_eq!(
        session.resolved_button_mask() & NesButton::Left.mask(),
        NesButton::Left.mask()
    );
    assert_eq!(session.resolved_button_mask() & NesButton::Right.mask(), 0);
}
