mod support;

use std::path::PathBuf;

use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::shell::{
    AudioInitError, LoadedRom, PauseState, RuntimeAudio, RuntimeMenuMode, RuntimePreferences,
    RuntimeSession,
};
use winit::keyboard::KeyCode;

use support::assertions::{
    assert_audio_has_activity, assert_audio_samples_eq, assert_audio_silent,
};

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
            source_path: PathBuf::from("audio-fixture.nes"),
            mapper_id: 0,
            title: Some(String::from("audio-fixture")),
        },
        mapper0_cartridge(),
    )
}

#[test]
fn audio_init_failure_stays_recoverable_for_runtime_sessions() {
    let mut session = runtime_session();

    let error = session
        .start_audio_output_with(|_| Err(AudioInitError::NoOutputDevice))
        .expect_err("audio startup should fail for the injected backend");

    assert_eq!(
        error.diagnostic_message(),
        "RustNES could not start audio playback: no default audio output device is available."
    );
    assert!(!session.audio_output_available());

    session.open_pause_menu();
    assert_eq!(session.pause_state(), PauseState::Paused);
    session.resume();
    assert_eq!(session.pause_state(), PauseState::Running);
}

#[test]
fn offline_audio_applies_master_volume_and_mute_immediately() {
    let mut audio = RuntimeAudio::without_output(44_100);
    audio.apply_preferences(
        RuntimePreferences {
            master_volume: 0.5,
            muted: false,
        },
        false,
    );
    audio.push_samples(&[1.0, -1.0]);

    assert_audio_samples_eq(
        &audio.render_offline(2, 2),
        &[0.5, 0.5, -0.5, -0.5],
        1.0e-6,
        "offline playback should honor master volume",
    );

    audio.apply_preferences(
        RuntimePreferences {
            master_volume: 0.5,
            muted: true,
        },
        false,
    );
    audio.push_samples(&[1.0]);

    assert_audio_silent(
        &audio.render_offline(1, 2),
        1.0e-6,
        "muted offline playback should be silent",
    );
}

#[test]
fn pause_silence_discards_stale_buffer_before_resume() {
    let mut audio = RuntimeAudio::without_output(44_100);
    audio.apply_preferences(RuntimePreferences::default(), false);
    audio.push_samples(&[0.8, 0.6]);

    audio.apply_preferences(RuntimePreferences::default(), true);
    assert_audio_silent(
        &audio.render_offline(2, 2),
        1.0e-6,
        "pause should clear stale buffered samples",
    );

    audio.apply_preferences(RuntimePreferences::default(), false);
    assert_audio_silent(
        &audio.render_offline(2, 2),
        1.0e-6,
        "resume should not replay discarded buffered samples",
    );
}

#[test]
fn paused_audio_controls_toggle_mute_and_adjust_volume() {
    let mut session = runtime_session();
    session.open_pause_menu();

    for _ in 0..4 {
        session
            .handle_runtime_key(KeyCode::ArrowDown, true, false)
            .expect("pause menu navigation should work");
    }
    assert!(matches!(
        session.menu_mode(),
        RuntimeMenuMode::PauseMenu { .. }
    ));

    session
        .handle_runtime_key(KeyCode::Enter, true, false)
        .expect("enter should open audio controls");
    assert_eq!(session.menu_mode(), RuntimeMenuMode::AudioControls);

    session
        .handle_runtime_key(KeyCode::KeyM, true, false)
        .expect("mute toggle should work while paused");
    assert!(session.preferences().muted);

    session
        .handle_runtime_key(KeyCode::ArrowLeft, true, false)
        .expect("volume down should work while paused");
    assert!(session.preferences().master_volume < 1.0);

    let volume_after_left = session.preferences().master_volume;
    session
        .handle_runtime_key(KeyCode::ArrowRight, true, false)
        .expect("volume up should work while paused");
    assert!(session.preferences().master_volume > volume_after_left);

    let mut offline = RuntimeAudio::without_output(44_100);
    offline.apply_preferences(session.preferences(), true);
    offline.push_samples(&[0.75, -0.75]);
    assert_audio_silent(
        &offline.render_offline(2, 2),
        1.0e-6,
        "paused audio controls should keep playback silent",
    );

    session
        .handle_runtime_key(KeyCode::KeyM, true, false)
        .expect("mute toggle should work while paused");
    assert!(!session.preferences().muted);

    offline.apply_preferences(session.preferences(), false);
    offline.push_samples(&[0.75, -0.75]);
    assert_audio_has_activity(
        &offline.render_offline(2, 2),
        0.1,
        "resumed audio should emit scaled samples",
    );
}
