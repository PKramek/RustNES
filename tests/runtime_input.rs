mod support;

use RustNES::shell::{NesButton, RuntimeSession};
use winit::keyboard::KeyCode;

use support::runtime_script::{build_ines_rom, load_runtime_session_from_bytes};

fn load_runtime_session() -> RuntimeSession {
    load_runtime_session_from_bytes("runtime-input", &build_ines_rom(1, 1, 0, 0))
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
