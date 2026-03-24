mod app;
mod launcher;
mod load_rom;
mod runtime;
mod trace;

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;

pub use app::{App, AppState, LoadFailure, LoadedSession, OpenRomOutcome};
pub use launcher::Launcher;
pub use load_rom::{LoadRomError, LoadedRom, load_rom_from_path};
pub use runtime::{
    AudioInitError, InputBindings, InputState, NesButton, PauseMenuAction, PauseState,
    RuntimeActionError, RuntimeAudio, RuntimeBootstrapError, RuntimeMenuMode, RuntimePreferences,
    RuntimeSession,
};
pub use trace::TraceOptions;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootOptions {
    pub initial_rom: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellCommand {
    Launcher(BootOptions),
    Trace(TraceOptions),
}

impl BootOptions {
    pub fn from_env() -> Self {
        let mut args = std::env::args_os();
        let _ = args.next();
        Self {
            initial_rom: args.next().map(PathBuf::from),
        }
    }
}

impl ShellCommand {
    pub fn from_env() -> Result<Self> {
        let args = std::env::args_os().collect::<Vec<_>>();
        Self::from_args(args)
    }

    pub fn from_args(args: Vec<OsString>) -> Result<Self> {
        match args.get(1).and_then(|value| value.to_str()) {
            Some("trace") => Ok(Self::Trace(TraceOptions::from_args(args)?)),
            _ => Ok(Self::Launcher(BootOptions {
                initial_rom: initial_rom_arg(args),
            })),
        }
    }
}

pub fn run(command: ShellCommand) -> Result<()> {
    match command {
        ShellCommand::Launcher(options) => {
            let launcher = Launcher::boot(options);

            match launcher.into_app().into_state() {
                AppState::Loaded(session) => {
                    if let Err(error) = runtime::run(*session) {
                        eprintln!("{}", error.diagnostic_message());
                    }
                }
                AppState::LoadFailed(failure) => {
                    eprintln!("{}", failure.message);
                }
                AppState::Launcher | AppState::Loading(_) => {}
            }

            Ok(())
        }
        ShellCommand::Trace(options) => trace::run_trace(options),
    }
}

pub fn initial_rom_arg(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut iter = args.into_iter();
    let _ = iter.next();
    iter.next().map(PathBuf::from)
}
