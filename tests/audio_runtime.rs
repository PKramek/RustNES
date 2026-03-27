use std::path::PathBuf;

use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::shell::{
    AudioInitError, LoadedRom, PauseState, RuntimeAudio, RuntimePreferences, RuntimeSession,
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

    assert_eq!(audio.render_offline(2, 2), vec![0.5, 0.5, -0.5, -0.5]);

    audio.apply_preferences(
        RuntimePreferences {
            master_volume: 0.5,
            muted: true,
        },
        false,
    );
    audio.push_samples(&[1.0]);

    assert_eq!(audio.render_offline(1, 2), vec![0.0, 0.0]);
}

#[test]
fn pause_silence_discards_stale_buffer_before_resume() {
    let mut audio = RuntimeAudio::without_output(44_100);
    audio.apply_preferences(RuntimePreferences::default(), false);
    audio.push_samples(&[0.8, 0.6]);

    audio.apply_preferences(RuntimePreferences::default(), true);
    assert_eq!(audio.render_offline(2, 2), vec![0.0, 0.0, 0.0, 0.0]);

    audio.apply_preferences(RuntimePreferences::default(), false);
    assert_eq!(audio.render_offline(2, 2), vec![0.0, 0.0, 0.0, 0.0]);
}
