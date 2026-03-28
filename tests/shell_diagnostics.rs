mod support;

use RustNES::shell::{
    AppState, BootOptions, Launcher, RuntimeBootstrapError, ShellDiagnostic, load_rom_from_path,
};

use support::runtime_script::{build_ines_rom, write_rom_fixture};
use support::unique_temp_path;

#[test]
fn missing_rom_startup_failure_exposes_shell_diagnostic() {
    let missing = unique_temp_path("shell-diagnostic-missing", "nes");
    let launcher = Launcher::boot(BootOptions {
        initial_rom: Some(missing.clone()),
    });

    let failure = match launcher.state() {
        AppState::LoadFailed(failure) => failure,
        state => panic!("expected load failure, got {state:?}"),
    };

    let diagnostic = ShellDiagnostic::from_load_failure(failure);
    assert_eq!(diagnostic.title, "ROM Load Failed");
    assert!(diagnostic.message.contains("could not read ROM"));
    assert!(
        diagnostic
            .detail
            .expect("detail should exist")
            .contains(&missing.display().to_string())
    );
}

#[test]
fn invalid_rom_parse_failure_stays_calm() {
    let path = write_rom_fixture("shell-diagnostic-invalid", &[0, 1, 2, 3]);
    let error = load_rom_from_path(&path).expect_err("invalid rom should fail");

    assert!(error.diagnostic_message().contains("could not be loaded"));

    let failure = RustNES::shell::LoadFailure {
        attempted_path: path.clone(),
        message: error.diagnostic_message(),
        error,
    };
    let diagnostic = ShellDiagnostic::from_load_failure(&failure);
    assert!(diagnostic.message.contains("could not be loaded"));
}

#[test]
fn unsupported_rom_format_diagnostic_remains_explicit() {
    let path = write_rom_fixture(
        "shell-diagnostic-unsupported",
        &build_ines_rom(1, 1, 0x10, 0),
    );
    let error = load_rom_from_path(&path).expect_err("unsupported mapper should fail");

    let failure = RustNES::shell::LoadFailure {
        attempted_path: path,
        message: error.diagnostic_message(),
        error,
    };
    let diagnostic = ShellDiagnostic::from_load_failure(&failure);
    assert!(
        diagnostic.message.contains("unsupported ROM format")
            || diagnostic.message.contains("unsupported mapper")
    );
}

#[test]
fn runtime_bootstrap_diagnostic_uses_shared_surface() {
    let diagnostic =
        ShellDiagnostic::from_runtime_bootstrap_error(&RuntimeBootstrapError::Pixels {
            source: pixels::Error::InvalidTexture(pixels::TextureError::TextureWidth(0)),
        });

    assert_eq!(diagnostic.title, "Runtime View Failed to Start");
    assert!(
        diagnostic
            .message
            .contains("could not start the runtime view")
    );
}
