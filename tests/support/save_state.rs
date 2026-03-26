use std::fs;
use std::path::PathBuf;

use super::{rom_bytes, unique_temp_path};

#[derive(Debug)]
pub struct SaveStateFixture {
    root: PathBuf,
    pub rom_path: PathBuf,
    pub slot_path: PathBuf,
}

impl SaveStateFixture {
    pub fn from_program(label: &str, program: &[(u16, u8)], reset: u16) -> Self {
        Self::from_rom_bytes(label, &rom_bytes(program, reset, reset, reset))
    }

    pub fn from_rom_bytes(label: &str, rom: &[u8]) -> Self {
        let root = unique_temp_path(label, "tmp");
        fs::create_dir_all(&root).expect("save-state fixture root should create");

        let rom_path = root.join("fixture.nes");
        let slot_path = root.join("quick-save-slot.bin");

        fs::write(&rom_path, rom).expect("fixture ROM should write");

        Self {
            root,
            rom_path,
            slot_path,
        }
    }

    pub fn slot_exists(&self) -> bool {
        self.slot_path.exists()
    }

    pub fn write_slot(&self, bytes: &[u8]) {
        fs::write(&self.slot_path, bytes).expect("save-state slot should write");
    }

    pub fn read_slot(&self) -> Vec<u8> {
        fs::read(&self.slot_path).expect("save-state slot should read")
    }
}

impl Drop for SaveStateFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.slot_path);
        let _ = fs::remove_file(&self.rom_path);
        let _ = fs::remove_dir(&self.root);
    }
}
