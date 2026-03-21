use super::{AddressingMode, StepRecord, opcode_meta};

pub fn format_trace_lines(records: &[StepRecord]) -> String {
    let mut output = String::new();
    for record in records {
        output.push_str(&format_trace_line(record));
        output.push('\n');
    }
    output
}

pub fn format_trace_line(record: &StepRecord) -> String {
    let meta = opcode_meta(record.opcode);
    let bytes = format_bytes(record);
    let operand = format_operand(meta.mode, record);
    let (scanline, dot) = ppu_position(record.cyc_before);

    format!(
        "{:04X}  {:<8} {:<32} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:>3},{:>3} CYC:{}",
        record.pc_before,
        bytes,
        if operand.is_empty() {
            meta.mnemonic.to_string()
        } else {
            format!("{} {}", meta.mnemonic, operand)
        },
        record.a,
        record.x,
        record.y,
        record.status,
        record.sp,
        scanline,
        dot,
        record.cyc_before,
    )
}

fn format_bytes(record: &StepRecord) -> String {
    let mut bytes = String::new();
    for (index, value) in record.bytes.iter().enumerate().take(record.byte_len as usize) {
        if index > 0 {
            bytes.push(' ');
        }
        bytes.push_str(&format!("{:02X}", value));
    }
    bytes
}

fn format_operand(mode: AddressingMode, record: &StepRecord) -> String {
    match mode {
        AddressingMode::Implied => String::new(),
        AddressingMode::Accumulator => String::from("A"),
        AddressingMode::Immediate => format!("#${:02X}", record.bytes[1]),
        AddressingMode::ZeroPage => match (record.operand_addr, record.operand_value) {
            (Some(addr), Some(value)) => format!("${:02X} = {:02X}", addr as u8, value),
            _ => format!("${:02X}", record.bytes[1]),
        },
        AddressingMode::ZeroPageX => match (record.operand_addr, record.operand_value) {
            (Some(addr), Some(value)) => {
                format!("${:02X},X @ {:02X} = {:02X}", record.bytes[1], addr as u8, value)
            }
            _ => format!("${:02X},X", record.bytes[1]),
        },
        AddressingMode::ZeroPageY => match (record.operand_addr, record.operand_value) {
            (Some(addr), Some(value)) => {
                format!("${:02X},Y @ {:02X} = {:02X}", record.bytes[1], addr as u8, value)
            }
            _ => format!("${:02X},Y", record.bytes[1]),
        },
        AddressingMode::Relative => record
            .branch_target
            .map(|target| format!("${:04X}", target))
            .unwrap_or_default(),
        AddressingMode::Absolute => {
            let base = u16::from_le_bytes([record.bytes[1], record.bytes[2]]);
            match record.operand_value {
                Some(value) => format!("${:04X} = {:02X}", base, value),
                None => format!("${:04X}", base),
            }
        }
        AddressingMode::AbsoluteX => {
            let base = u16::from_le_bytes([record.bytes[1], record.bytes[2]]);
            match (record.operand_addr, record.operand_value) {
                (Some(addr), Some(value)) => format!("${:04X},X @ {:04X} = {:02X}", base, addr, value),
                (Some(addr), None) => format!("${:04X},X @ {:04X}", base, addr),
                _ => format!("${:04X},X", base),
            }
        }
        AddressingMode::AbsoluteY => {
            let base = u16::from_le_bytes([record.bytes[1], record.bytes[2]]);
            match (record.operand_addr, record.operand_value) {
                (Some(addr), Some(value)) => format!("${:04X},Y @ {:04X} = {:02X}", base, addr, value),
                (Some(addr), None) => format!("${:04X},Y @ {:04X}", base, addr),
                _ => format!("${:04X},Y", base),
            }
        }
        AddressingMode::Indirect => match (record.pointer_addr, record.pointer_value) {
            (Some(pointer), Some(target)) => format!("(${:04X}) = {:04X}", pointer, target),
            _ => format!("(${:04X})", u16::from_le_bytes([record.bytes[1], record.bytes[2]])),
        },
        AddressingMode::IndirectX => match (record.pointer_addr, record.pointer_value, record.operand_value) {
            (Some(pointer), Some(target), Some(value)) => format!(
                "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                record.bytes[1],
                pointer as u8,
                target,
                value
            ),
            _ => format!("(${:02X},X)", record.bytes[1]),
        },
        AddressingMode::IndirectY => match (
            record.pointer_addr,
            record.pointer_value,
            record.operand_addr,
            record.operand_value,
        ) {
            (Some(pointer), Some(base), Some(addr), Some(value)) => format!(
                "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                pointer as u8,
                base,
                addr,
                value
            ),
            _ => format!("(${:02X}),Y", record.bytes[1]),
        },
    }
}

fn ppu_position(cpu_cycles: u64) -> (u16, u16) {
    let ppu_cycles = cpu_cycles.saturating_mul(3);
    let scanline = ((ppu_cycles / 341) % 262) as u16;
    let dot = (ppu_cycles % 341) as u16;
    (scanline, dot)
}