use std::path::{Path, PathBuf};

use crate::core::cartridge::Cartridge;

use super::{load_rom_from_path, LoadRomError, LoadedRom};

#[derive(Debug)]
pub enum AppState {
    Launcher,
    Loading(PathBuf),
    Loaded(LoadedSession),
    LoadFailed(LoadFailure),
}

#[derive(Debug)]
pub struct LoadedSession {
    pub rom: LoadedRom,
    pub cartridge: Cartridge,
}

#[derive(Debug)]
pub struct LoadFailure {
    pub attempted_path: PathBuf,
    pub error: LoadRomError,
    pub message: String,
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

    pub fn open_path_with_confirmation<F>(&mut self, path: PathBuf, confirm_replace: F) -> OpenRomOutcome
    where
        F: FnOnce(&LoadedRom, &Path) -> bool,
    {
        if let AppState::Loaded(current) = &self.state {
            if !confirm_replace(&current.rom, &path) {
                return OpenRomOutcome::CancelledReplace;
            }
        }

        self.state = AppState::Loading(path.clone());

        match load_rom_from_path(&path) {
            Ok((rom, cartridge)) => {
                self.state = AppState::Loaded(LoadedSession { rom, cartridge });
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