use std::ffi::OsString;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::core::console::Console;
use crate::core::cpu::format_trace_line;

use super::load_rom_from_path;

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
                    let value = iter
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for --pc"))?;
                    start_pc = Some(parse_hex_u16(&value.to_string_lossy())?);
                }
                Some("--output") => {
                    output =
                        Some(PathBuf::from(iter.next().ok_or_else(|| {
                            anyhow::anyhow!("missing value for --output")
                        })?));
                }
                Some("--max-instructions") => {
                    let value = iter
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("missing value for --max-instructions"))?;
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

pub fn run_trace(options: TraceOptions) -> Result<()> {
    let (_, cartridge) = load_rom_from_path(&options.rom_path)
        .map_err(|error| anyhow::anyhow!(error.diagnostic_message()))?;
    let mut console = Console::new(cartridge);
    console.reset();

    if let Some(pc) = options.start_pc {
        console.cpu_mut().pc = pc;
    }

    if let Some(output_path) = &options.output {
        let file = std::fs::File::create(output_path)
            .with_context(|| format!("failed to create trace output {}", output_path.display()))?;
        let mut writer = BufWriter::new(file);
        emit_trace(&mut writer, &mut console, options.max_instructions)?;
        writer.flush()?;
    } else {
        let stdout = std::io::stdout();
        let mut writer = stdout.lock();
        emit_trace(&mut writer, &mut console, options.max_instructions)?;
        writer.flush()?;
    }

    Ok(())
}

fn parse_hex_u16(text: &str) -> Result<u16> {
    let normalized = text.trim().trim_start_matches("0x").trim_start_matches('$');
    Ok(u16::from_str_radix(normalized, 16)?)
}

fn emit_trace(
    writer: &mut impl Write,
    console: &mut Console,
    max_instructions: Option<usize>,
) -> Result<()> {
    let limit = max_instructions.unwrap_or(10_000);
    for _ in 0..limit {
        let record = console.step_instruction()?;
        writeln!(writer, "{}", format_trace_line(&record))?;
        if max_instructions.is_none() && record.opcode == 0x00 {
            break;
        }
    }

    Ok(())
}
