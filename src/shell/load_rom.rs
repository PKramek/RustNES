use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::core::cartridge::{Cartridge, CartridgeError, load_cartridge_from_bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRom {
    pub source_path: PathBuf,
    pub mapper_id: u16,
    pub title: Option<String>,
}

#[derive(Debug, Error)]
pub enum LoadRomError {
    #[error("failed to read ROM from {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to load ROM from {path}: {source}")]
    Cartridge {
        path: PathBuf,
        #[source]
        source: CartridgeError,
    },
}

impl LoadRomError {
    pub fn diagnostic_message(&self) -> String {
        match self {
            Self::Io { path, source } => {
                format!(
                    "RustNES could not read ROM at {}: {}.",
                    path.display(),
                    source
                )
            }
            Self::Cartridge {
                source: CartridgeError::UnsupportedMapper { mapper, reason },
                ..
            } => format!("This ROM uses unsupported mapper {mapper}. {reason}"),
            Self::Cartridge { source, .. } => {
                format!("This ROM could not be loaded: {source}.")
            }
        }
    }
}

pub fn load_rom_from_path(path: impl AsRef<Path>) -> Result<(LoadedRom, Cartridge), LoadRomError> {
    let source_path = path.as_ref().to_path_buf();
    let bytes = std::fs::read(&source_path).map_err(|source| LoadRomError::Io {
        path: source_path.clone(),
        source,
    })?;
    let cartridge =
        load_cartridge_from_bytes(&bytes).map_err(|source| LoadRomError::Cartridge {
            path: source_path.clone(),
            source,
        })?;

    let metadata = LoadedRom {
        source_path,
        mapper_id: cartridge.header().mapper_id,
        title: None,
    };

    Ok((metadata, cartridge))
}
