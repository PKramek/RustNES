use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceOptions {
    pub rom_path: PathBuf,
    pub start_pc: Option<u16>,
    pub output: Option<PathBuf>,
    pub max_instructions: Option<usize>,
}

impl TraceOptions {
    pub fn from_args(args: Vec<OsString>) -> Result<Self> {
        let mut rom_path = None;
        let mut start_pc = None;
        let mut output = None;
        let mut max_instructions = None;

        let mut iter = args.into_iter();
        let _ = iter.next();
        let _ = iter.next();

        while let Some(arg) = iter.next() {
            match arg.to_str() {
                Some("--pc") => {
                    let value = iter.next().ok_or_else(|| anyhow::anyhow!("missing value for --pc"))?;
                    let text = value.to_string_lossy();
                    start_pc = Some(u16::from_str_radix(text.trim_start_matches("0x"), 16)?);
                }
                Some("--output") => {
                    output = Some(PathBuf::from(iter.next().ok_or_else(|| anyhow::anyhow!("missing value for --output"))?));
                }
                Some("--max-instructions") => {
                    let value = iter.next().ok_or_else(|| anyhow::anyhow!("missing value for --max-instructions"))?;
                    max_instructions = Some(value.to_string_lossy().parse()?);
                }
                Some(flag) if flag.starts_with("--") => bail!("unsupported trace flag: {flag}"),
                _ => {
                    if rom_path.is_none() {
                        rom_path = Some(PathBuf::from(arg));
                    } else {
                        bail!("trace accepts only one ROM path");
                    }
                }
            }
        }

        Ok(Self {
            rom_path: rom_path.ok_or_else(|| anyhow::anyhow!("trace requires a ROM path"))?,
            start_pc,
            output,
            max_instructions,
        })
    }
}

pub fn run_trace(_options: TraceOptions) -> Result<()> {
    Ok(())
}