use std::path::{Path, PathBuf};

use super::diagnostics::ShellDiagnostic;
use super::{LoadRomError, LoadedRom, RuntimeSession, load_rom_from_path};

#[derive(Debug)]
pub enum AppState {
    Launcher,
    Loading(PathBuf),
    Loaded(Box<RuntimeSession>),
    LoadFailed(LoadFailure),
}

pub type LoadedSession = Box<RuntimeSession>;

#[derive(Debug)]
pub struct LoadFailure {
    pub attempted_path: PathBuf,
    pub error: LoadRomError,
    pub message: String,
}

impl LoadFailure {
    pub fn diagnostic(&self) -> ShellDiagnostic {
        ShellDiagnostic::from_load_failure(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenRomOutcome {
    Loaded,
    CancelledReplace,
    Failed,
}

pub struct App {
    state: AppState,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            state: AppState::Launcher,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn into_state(self) -> AppState {
        self.state
    }

    pub fn open_path_with_confirmation<F>(
        &mut self,
        path: PathBuf,
        confirm_replace: F,
    ) -> OpenRomOutcome
    where
        F: FnOnce(&LoadedRom, &Path) -> bool,
    {
        if let AppState::Loaded(current) = &self.state
            && !confirm_replace(&current.rom, &path)
        {
            return OpenRomOutcome::CancelledReplace;
        }

        self.state = AppState::Loading(path.clone());

        match load_rom_from_path(&path) {
            Ok((rom, cartridge)) => {
                self.state = AppState::Loaded(Box::new(RuntimeSession::new(rom, cartridge)));
                OpenRomOutcome::Loaded
            }
            Err(error) => {
                let message = error.diagnostic_message();
                self.state = AppState::LoadFailed(LoadFailure {
                    attempted_path: path,
                    error,
                    message,
                });
                OpenRomOutcome::Failed
            }
        }
    }

    pub fn dismiss_error(&mut self) {
        if matches!(self.state, AppState::LoadFailed(_)) {
            self.state = AppState::Launcher;
        }
    }
}
