mod app;
mod launcher;
mod load_rom;

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;

pub use app::{App, AppState, LoadFailure, LoadedSession, OpenRomOutcome};
pub use launcher::Launcher;
pub use load_rom::{load_rom_from_path, LoadRomError, LoadedRom};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootOptions {
    pub initial_rom: Option<PathBuf>,
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

pub fn run(options: BootOptions) -> Result<()> {
    let launcher = Launcher::boot(options);

    if let AppState::LoadFailed(failure) = launcher.state() {
        eprintln!("{}", failure.message);
    }

    Ok(())
}

pub fn initial_rom_arg(args: impl IntoIterator<Item = OsString>) -> Option<PathBuf> {
    let mut iter = args.into_iter();
    let _ = iter.next();
    iter.next().map(PathBuf::from)
}