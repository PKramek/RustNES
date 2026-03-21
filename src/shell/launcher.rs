use std::path::{Path, PathBuf};

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use super::{App, AppState, BootOptions, LoadedRom, OpenRomOutcome};

pub struct Launcher {
    app: App,
}

impl Launcher {
    pub fn boot(options: BootOptions) -> Self {
        let mut app = App::new();

        if let Some(initial_rom) = options.initial_rom {
            let _ = app.open_path_with_confirmation(initial_rom, |_current, _next| true);
        }

        Self { app }
    }

    pub fn state(&self) -> &AppState {
        self.app.state()
    }

    pub fn open_path_with_confirmation<F>(&mut self, path: PathBuf, confirm_replace: F) -> OpenRomOutcome
    where
        F: FnOnce(&LoadedRom, &Path) -> bool,
    {
        self.app.open_path_with_confirmation(path, confirm_replace)
    }

    pub fn dismiss_error(&mut self) {
        self.app.dismiss_error();
    }

    #[allow(dead_code)]
    pub fn pick_rom_path() -> Option<PathBuf> {
        FileDialog::new().add_filter("NES ROM", &["nes"]).pick_file()
    }

    #[allow(dead_code)]
    pub fn confirm_replace(current: &LoadedRom, next: &Path) -> bool {
        matches!(
            MessageDialog::new()
                .set_level(MessageLevel::Warning)
                .set_title("Replace loaded ROM?")
                .set_description(format!(
                    "Replace the current session from {} with {}?",
                    current.source_path.display(),
                    next.display()
                ))
                .set_buttons(MessageButtons::YesNo)
                .show(),
            MessageDialogResult::Yes
        )
    }

    #[allow(dead_code)]
    pub fn show_load_error(message: &str) {
        let _ = MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title("ROM Load Error")
            .set_description(message)
            .set_buttons(MessageButtons::Ok)
            .show();
    }
}