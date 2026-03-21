use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CartridgeError {
    #[error("invalid iNES magic bytes")]
    InvalidMagic,
    #[error("unsupported ROM format: {format}")]
    UnsupportedFormat { format: &'static str },
    #[error("dirty or ambiguous iNES header: {reason}")]
    DirtyHeader { reason: String },
    #[error("truncated ROM: expected at least {expected} bytes but found {actual}")]
    TruncatedRom { expected: usize, actual: usize },
    #[error("unsupported mapper {mapper}: {reason}")]
    UnsupportedMapper { mapper: u16, reason: String },
    #[error("unsupported cartridge layout: {reason}")]
    UnsupportedCartridgeLayout { reason: String },
}