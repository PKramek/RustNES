use super::AddressingMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpcodeMeta {
    pub mnemonic: &'static str,
    pub mode: AddressingMode,
    pub bytes: u8,
    pub base_cycles: u8,
    pub official: bool,
}

const fn opcode(
    mnemonic: &'static str,
    mode: AddressingMode,
    bytes: u8,
    base_cycles: u8,
    official: bool,
) -> OpcodeMeta {
    OpcodeMeta {
        mnemonic,
        mode,
        bytes,
        base_cycles,
        official,
    }
}

const fn illegal() -> OpcodeMeta {
    opcode("???", AddressingMode::Implied, 1, 0, false)
}

const fn unofficial_nop(mode: AddressingMode, bytes: u8, base_cycles: u8) -> OpcodeMeta {
    opcode("*NOP", mode, bytes, base_cycles, false)
}

const fn unofficial(
    mnemonic: &'static str,
    mode: AddressingMode,
    bytes: u8,
    base_cycles: u8,
) -> OpcodeMeta {
    opcode(mnemonic, mode, bytes, base_cycles, false)
}

const fn build_table() -> [OpcodeMeta; 256] {
    let mut table = [illegal(); 256];

    table[0x00] = opcode("BRK", AddressingMode::Implied, 1, 7, true);
    table[0x01] = opcode("ORA", AddressingMode::IndirectX, 2, 6, true);
    table[0x03] = unofficial("*SLO", AddressingMode::IndirectX, 2, 8);
    table[0x04] = unofficial_nop(AddressingMode::ZeroPage, 2, 3);
    table[0x05] = opcode("ORA", AddressingMode::ZeroPage, 2, 3, true);
    table[0x06] = opcode("ASL", AddressingMode::ZeroPage, 2, 5, true);
    table[0x07] = unofficial("*SLO", AddressingMode::ZeroPage, 2, 5);
    table[0x08] = opcode("PHP", AddressingMode::Implied, 1, 3, true);
    table[0x09] = opcode("ORA", AddressingMode::Immediate, 2, 2, true);
    table[0x0A] = opcode("ASL", AddressingMode::Accumulator, 1, 2, true);
    table[0x0C] = unofficial_nop(AddressingMode::Absolute, 3, 4);
    table[0x0D] = opcode("ORA", AddressingMode::Absolute, 3, 4, true);
    table[0x0E] = opcode("ASL", AddressingMode::Absolute, 3, 6, true);
    table[0x0F] = unofficial("*SLO", AddressingMode::Absolute, 3, 6);
    table[0x10] = opcode("BPL", AddressingMode::Relative, 2, 2, true);
    table[0x11] = opcode("ORA", AddressingMode::IndirectY, 2, 5, true);
    table[0x13] = unofficial("*SLO", AddressingMode::IndirectY, 2, 8);
    table[0x14] = unofficial_nop(AddressingMode::ZeroPageX, 2, 4);
    table[0x15] = opcode("ORA", AddressingMode::ZeroPageX, 2, 4, true);
    table[0x16] = opcode("ASL", AddressingMode::ZeroPageX, 2, 6, true);
    table[0x17] = unofficial("*SLO", AddressingMode::ZeroPageX, 2, 6);
    table[0x18] = opcode("CLC", AddressingMode::Implied, 1, 2, true);
    table[0x19] = opcode("ORA", AddressingMode::AbsoluteY, 3, 4, true);
    table[0x1A] = unofficial_nop(AddressingMode::Implied, 1, 2);
    table[0x1B] = unofficial("*SLO", AddressingMode::AbsoluteY, 3, 7);
    table[0x1C] = unofficial_nop(AddressingMode::AbsoluteX, 3, 4);
    table[0x1D] = opcode("ORA", AddressingMode::AbsoluteX, 3, 4, true);
    table[0x1E] = opcode("ASL", AddressingMode::AbsoluteX, 3, 7, true);
    table[0x1F] = unofficial("*SLO", AddressingMode::AbsoluteX, 3, 7);

    table[0x20] = opcode("JSR", AddressingMode::Absolute, 3, 6, true);
    table[0x21] = opcode("AND", AddressingMode::IndirectX, 2, 6, true);
    table[0x23] = unofficial("*RLA", AddressingMode::IndirectX, 2, 8);
    table[0x24] = opcode("BIT", AddressingMode::ZeroPage, 2, 3, true);
    table[0x25] = opcode("AND", AddressingMode::ZeroPage, 2, 3, true);
    table[0x26] = opcode("ROL", AddressingMode::ZeroPage, 2, 5, true);
    table[0x27] = unofficial("*RLA", AddressingMode::ZeroPage, 2, 5);
    table[0x28] = opcode("PLP", AddressingMode::Implied, 1, 4, true);
    table[0x29] = opcode("AND", AddressingMode::Immediate, 2, 2, true);
    table[0x2A] = opcode("ROL", AddressingMode::Accumulator, 1, 2, true);
    table[0x2C] = opcode("BIT", AddressingMode::Absolute, 3, 4, true);
    table[0x2D] = opcode("AND", AddressingMode::Absolute, 3, 4, true);
    table[0x2E] = opcode("ROL", AddressingMode::Absolute, 3, 6, true);
    table[0x2F] = unofficial("*RLA", AddressingMode::Absolute, 3, 6);
    table[0x30] = opcode("BMI", AddressingMode::Relative, 2, 2, true);
    table[0x31] = opcode("AND", AddressingMode::IndirectY, 2, 5, true);
    table[0x33] = unofficial("*RLA", AddressingMode::IndirectY, 2, 8);
    table[0x34] = unofficial_nop(AddressingMode::ZeroPageX, 2, 4);
    table[0x35] = opcode("AND", AddressingMode::ZeroPageX, 2, 4, true);
    table[0x36] = opcode("ROL", AddressingMode::ZeroPageX, 2, 6, true);
    table[0x37] = unofficial("*RLA", AddressingMode::ZeroPageX, 2, 6);
    table[0x38] = opcode("SEC", AddressingMode::Implied, 1, 2, true);
    table[0x39] = opcode("AND", AddressingMode::AbsoluteY, 3, 4, true);
    table[0x3A] = unofficial_nop(AddressingMode::Implied, 1, 2);
    table[0x3B] = unofficial("*RLA", AddressingMode::AbsoluteY, 3, 7);
    table[0x3C] = unofficial_nop(AddressingMode::AbsoluteX, 3, 4);
    table[0x3D] = opcode("AND", AddressingMode::AbsoluteX, 3, 4, true);
    table[0x3E] = opcode("ROL", AddressingMode::AbsoluteX, 3, 7, true);
    table[0x3F] = unofficial("*RLA", AddressingMode::AbsoluteX, 3, 7);

    table[0x40] = opcode("RTI", AddressingMode::Implied, 1, 6, true);
    table[0x41] = opcode("EOR", AddressingMode::IndirectX, 2, 6, true);
    table[0x43] = unofficial("*SRE", AddressingMode::IndirectX, 2, 8);
    table[0x44] = unofficial_nop(AddressingMode::ZeroPage, 2, 3);
    table[0x45] = opcode("EOR", AddressingMode::ZeroPage, 2, 3, true);
    table[0x46] = opcode("LSR", AddressingMode::ZeroPage, 2, 5, true);
    table[0x47] = unofficial("*SRE", AddressingMode::ZeroPage, 2, 5);
    table[0x48] = opcode("PHA", AddressingMode::Implied, 1, 3, true);
    table[0x49] = opcode("EOR", AddressingMode::Immediate, 2, 2, true);
    table[0x4A] = opcode("LSR", AddressingMode::Accumulator, 1, 2, true);
    table[0x4C] = opcode("JMP", AddressingMode::Absolute, 3, 3, true);
    table[0x4D] = opcode("EOR", AddressingMode::Absolute, 3, 4, true);
    table[0x4E] = opcode("LSR", AddressingMode::Absolute, 3, 6, true);
    table[0x4F] = unofficial("*SRE", AddressingMode::Absolute, 3, 6);
    table[0x50] = opcode("BVC", AddressingMode::Relative, 2, 2, true);
    table[0x51] = opcode("EOR", AddressingMode::IndirectY, 2, 5, true);
    table[0x53] = unofficial("*SRE", AddressingMode::IndirectY, 2, 8);
    table[0x54] = unofficial_nop(AddressingMode::ZeroPageX, 2, 4);
    table[0x55] = opcode("EOR", AddressingMode::ZeroPageX, 2, 4, true);
    table[0x56] = opcode("LSR", AddressingMode::ZeroPageX, 2, 6, true);
    table[0x57] = unofficial("*SRE", AddressingMode::ZeroPageX, 2, 6);
    table[0x58] = opcode("CLI", AddressingMode::Implied, 1, 2, true);
    table[0x59] = opcode("EOR", AddressingMode::AbsoluteY, 3, 4, true);
    table[0x5A] = unofficial_nop(AddressingMode::Implied, 1, 2);
    table[0x5B] = unofficial("*SRE", AddressingMode::AbsoluteY, 3, 7);
    table[0x5C] = unofficial_nop(AddressingMode::AbsoluteX, 3, 4);
    table[0x5D] = opcode("EOR", AddressingMode::AbsoluteX, 3, 4, true);
    table[0x5E] = opcode("LSR", AddressingMode::AbsoluteX, 3, 7, true);
    table[0x5F] = unofficial("*SRE", AddressingMode::AbsoluteX, 3, 7);

    table[0x60] = opcode("RTS", AddressingMode::Implied, 1, 6, true);
    table[0x61] = opcode("ADC", AddressingMode::IndirectX, 2, 6, true);
    table[0x63] = unofficial("*RRA", AddressingMode::IndirectX, 2, 8);
    table[0x64] = unofficial_nop(AddressingMode::ZeroPage, 2, 3);
    table[0x65] = opcode("ADC", AddressingMode::ZeroPage, 2, 3, true);
    table[0x66] = opcode("ROR", AddressingMode::ZeroPage, 2, 5, true);
    table[0x67] = unofficial("*RRA", AddressingMode::ZeroPage, 2, 5);
    table[0x68] = opcode("PLA", AddressingMode::Implied, 1, 4, true);
    table[0x69] = opcode("ADC", AddressingMode::Immediate, 2, 2, true);
    table[0x6A] = opcode("ROR", AddressingMode::Accumulator, 1, 2, true);
    table[0x6C] = opcode("JMP", AddressingMode::Indirect, 3, 5, true);
    table[0x6D] = opcode("ADC", AddressingMode::Absolute, 3, 4, true);
    table[0x6E] = opcode("ROR", AddressingMode::Absolute, 3, 6, true);
    table[0x6F] = unofficial("*RRA", AddressingMode::Absolute, 3, 6);
    table[0x70] = opcode("BVS", AddressingMode::Relative, 2, 2, true);
    table[0x71] = opcode("ADC", AddressingMode::IndirectY, 2, 5, true);
    table[0x73] = unofficial("*RRA", AddressingMode::IndirectY, 2, 8);
    table[0x74] = unofficial_nop(AddressingMode::ZeroPageX, 2, 4);
    table[0x75] = opcode("ADC", AddressingMode::ZeroPageX, 2, 4, true);
    table[0x76] = opcode("ROR", AddressingMode::ZeroPageX, 2, 6, true);
    table[0x77] = unofficial("*RRA", AddressingMode::ZeroPageX, 2, 6);
    table[0x78] = opcode("SEI", AddressingMode::Implied, 1, 2, true);
    table[0x79] = opcode("ADC", AddressingMode::AbsoluteY, 3, 4, true);
    table[0x7A] = unofficial_nop(AddressingMode::Implied, 1, 2);
    table[0x7B] = unofficial("*RRA", AddressingMode::AbsoluteY, 3, 7);
    table[0x7C] = unofficial_nop(AddressingMode::AbsoluteX, 3, 4);
    table[0x7D] = opcode("ADC", AddressingMode::AbsoluteX, 3, 4, true);
    table[0x7E] = opcode("ROR", AddressingMode::AbsoluteX, 3, 7, true);
    table[0x7F] = unofficial("*RRA", AddressingMode::AbsoluteX, 3, 7);

    table[0x80] = unofficial_nop(AddressingMode::Immediate, 2, 2);
    table[0x81] = opcode("STA", AddressingMode::IndirectX, 2, 6, true);
    table[0x83] = unofficial("*SAX", AddressingMode::IndirectX, 2, 6);
    table[0x84] = opcode("STY", AddressingMode::ZeroPage, 2, 3, true);
    table[0x85] = opcode("STA", AddressingMode::ZeroPage, 2, 3, true);
    table[0x86] = opcode("STX", AddressingMode::ZeroPage, 2, 3, true);
    table[0x87] = unofficial("*SAX", AddressingMode::ZeroPage, 2, 3);
    table[0x88] = opcode("DEY", AddressingMode::Implied, 1, 2, true);
    table[0x8A] = opcode("TXA", AddressingMode::Implied, 1, 2, true);
    table[0x8C] = opcode("STY", AddressingMode::Absolute, 3, 4, true);
    table[0x8D] = opcode("STA", AddressingMode::Absolute, 3, 4, true);
    table[0x8E] = opcode("STX", AddressingMode::Absolute, 3, 4, true);
    table[0x8F] = unofficial("*SAX", AddressingMode::Absolute, 3, 4);
    table[0x90] = opcode("BCC", AddressingMode::Relative, 2, 2, true);
    table[0x91] = opcode("STA", AddressingMode::IndirectY, 2, 6, true);
    table[0x94] = opcode("STY", AddressingMode::ZeroPageX, 2, 4, true);
    table[0x95] = opcode("STA", AddressingMode::ZeroPageX, 2, 4, true);
    table[0x96] = opcode("STX", AddressingMode::ZeroPageY, 2, 4, true);
    table[0x97] = unofficial("*SAX", AddressingMode::ZeroPageY, 2, 4);
    table[0x98] = opcode("TYA", AddressingMode::Implied, 1, 2, true);
    table[0x99] = opcode("STA", AddressingMode::AbsoluteY, 3, 5, true);
    table[0x9A] = opcode("TXS", AddressingMode::Implied, 1, 2, true);
    table[0x9D] = opcode("STA", AddressingMode::AbsoluteX, 3, 5, true);

    table[0xA0] = opcode("LDY", AddressingMode::Immediate, 2, 2, true);
    table[0xA1] = opcode("LDA", AddressingMode::IndirectX, 2, 6, true);
    table[0xA2] = opcode("LDX", AddressingMode::Immediate, 2, 2, true);
    table[0xA3] = unofficial("*LAX", AddressingMode::IndirectX, 2, 6);
    table[0xA4] = opcode("LDY", AddressingMode::ZeroPage, 2, 3, true);
    table[0xA5] = opcode("LDA", AddressingMode::ZeroPage, 2, 3, true);
    table[0xA6] = opcode("LDX", AddressingMode::ZeroPage, 2, 3, true);
    table[0xA7] = unofficial("*LAX", AddressingMode::ZeroPage, 2, 3);
    table[0xA8] = opcode("TAY", AddressingMode::Implied, 1, 2, true);
    table[0xA9] = opcode("LDA", AddressingMode::Immediate, 2, 2, true);
    table[0xAA] = opcode("TAX", AddressingMode::Implied, 1, 2, true);
    table[0xAC] = opcode("LDY", AddressingMode::Absolute, 3, 4, true);
    table[0xAD] = opcode("LDA", AddressingMode::Absolute, 3, 4, true);
    table[0xAE] = opcode("LDX", AddressingMode::Absolute, 3, 4, true);
    table[0xAF] = unofficial("*LAX", AddressingMode::Absolute, 3, 4);
    table[0xB0] = opcode("BCS", AddressingMode::Relative, 2, 2, true);
    table[0xB1] = opcode("LDA", AddressingMode::IndirectY, 2, 5, true);
    table[0xB3] = unofficial("*LAX", AddressingMode::IndirectY, 2, 5);
    table[0xB4] = opcode("LDY", AddressingMode::ZeroPageX, 2, 4, true);
    table[0xB5] = opcode("LDA", AddressingMode::ZeroPageX, 2, 4, true);
    table[0xB6] = opcode("LDX", AddressingMode::ZeroPageY, 2, 4, true);
    table[0xB7] = unofficial("*LAX", AddressingMode::ZeroPageY, 2, 4);
    table[0xB8] = opcode("CLV", AddressingMode::Implied, 1, 2, true);
    table[0xB9] = opcode("LDA", AddressingMode::AbsoluteY, 3, 4, true);
    table[0xBA] = opcode("TSX", AddressingMode::Implied, 1, 2, true);
    table[0xBC] = opcode("LDY", AddressingMode::AbsoluteX, 3, 4, true);
    table[0xBD] = opcode("LDA", AddressingMode::AbsoluteX, 3, 4, true);
    table[0xBE] = opcode("LDX", AddressingMode::AbsoluteY, 3, 4, true);
    table[0xBF] = unofficial("*LAX", AddressingMode::AbsoluteY, 3, 4);

    table[0xC0] = opcode("CPY", AddressingMode::Immediate, 2, 2, true);
    table[0xC1] = opcode("CMP", AddressingMode::IndirectX, 2, 6, true);
    table[0xC3] = unofficial("*DCP", AddressingMode::IndirectX, 2, 8);
    table[0xC4] = opcode("CPY", AddressingMode::ZeroPage, 2, 3, true);
    table[0xC5] = opcode("CMP", AddressingMode::ZeroPage, 2, 3, true);
    table[0xC6] = opcode("DEC", AddressingMode::ZeroPage, 2, 5, true);
    table[0xC7] = unofficial("*DCP", AddressingMode::ZeroPage, 2, 5);
    table[0xC8] = opcode("INY", AddressingMode::Implied, 1, 2, true);
    table[0xC9] = opcode("CMP", AddressingMode::Immediate, 2, 2, true);
    table[0xCA] = opcode("DEX", AddressingMode::Implied, 1, 2, true);
    table[0xCC] = opcode("CPY", AddressingMode::Absolute, 3, 4, true);
    table[0xCD] = opcode("CMP", AddressingMode::Absolute, 3, 4, true);
    table[0xCE] = opcode("DEC", AddressingMode::Absolute, 3, 6, true);
    table[0xCF] = unofficial("*DCP", AddressingMode::Absolute, 3, 6);
    table[0xD0] = opcode("BNE", AddressingMode::Relative, 2, 2, true);
    table[0xD1] = opcode("CMP", AddressingMode::IndirectY, 2, 5, true);
    table[0xD3] = unofficial("*DCP", AddressingMode::IndirectY, 2, 8);
    table[0xD4] = unofficial_nop(AddressingMode::ZeroPageX, 2, 4);
    table[0xD5] = opcode("CMP", AddressingMode::ZeroPageX, 2, 4, true);
    table[0xD6] = opcode("DEC", AddressingMode::ZeroPageX, 2, 6, true);
    table[0xD7] = unofficial("*DCP", AddressingMode::ZeroPageX, 2, 6);
    table[0xD8] = opcode("CLD", AddressingMode::Implied, 1, 2, true);
    table[0xD9] = opcode("CMP", AddressingMode::AbsoluteY, 3, 4, true);
    table[0xDA] = unofficial_nop(AddressingMode::Implied, 1, 2);
    table[0xDB] = unofficial("*DCP", AddressingMode::AbsoluteY, 3, 7);
    table[0xDC] = unofficial_nop(AddressingMode::AbsoluteX, 3, 4);
    table[0xDD] = opcode("CMP", AddressingMode::AbsoluteX, 3, 4, true);
    table[0xDE] = opcode("DEC", AddressingMode::AbsoluteX, 3, 7, true);
    table[0xDF] = unofficial("*DCP", AddressingMode::AbsoluteX, 3, 7);

    table[0xE0] = opcode("CPX", AddressingMode::Immediate, 2, 2, true);
    table[0xE1] = opcode("SBC", AddressingMode::IndirectX, 2, 6, true);
    table[0xE3] = unofficial("*ISC", AddressingMode::IndirectX, 2, 8);
    table[0xE4] = opcode("CPX", AddressingMode::ZeroPage, 2, 3, true);
    table[0xE5] = opcode("SBC", AddressingMode::ZeroPage, 2, 3, true);
    table[0xE6] = opcode("INC", AddressingMode::ZeroPage, 2, 5, true);
    table[0xE7] = unofficial("*ISC", AddressingMode::ZeroPage, 2, 5);
    table[0xE8] = opcode("INX", AddressingMode::Implied, 1, 2, true);
    table[0xE9] = opcode("SBC", AddressingMode::Immediate, 2, 2, true);
    table[0xEA] = opcode("NOP", AddressingMode::Implied, 1, 2, true);
    table[0xEB] = unofficial("*SBC", AddressingMode::Immediate, 2, 2);
    table[0xEC] = opcode("CPX", AddressingMode::Absolute, 3, 4, true);
    table[0xED] = opcode("SBC", AddressingMode::Absolute, 3, 4, true);
    table[0xEE] = opcode("INC", AddressingMode::Absolute, 3, 6, true);
    table[0xEF] = unofficial("*ISC", AddressingMode::Absolute, 3, 6);
    table[0xF0] = opcode("BEQ", AddressingMode::Relative, 2, 2, true);
    table[0xF1] = opcode("SBC", AddressingMode::IndirectY, 2, 5, true);
    table[0xF3] = unofficial("*ISC", AddressingMode::IndirectY, 2, 8);
    table[0xF4] = unofficial_nop(AddressingMode::ZeroPageX, 2, 4);
    table[0xF5] = opcode("SBC", AddressingMode::ZeroPageX, 2, 4, true);
    table[0xF6] = opcode("INC", AddressingMode::ZeroPageX, 2, 6, true);
    table[0xF7] = unofficial("*ISC", AddressingMode::ZeroPageX, 2, 6);
    table[0xF8] = opcode("SED", AddressingMode::Implied, 1, 2, true);
    table[0xF9] = opcode("SBC", AddressingMode::AbsoluteY, 3, 4, true);
    table[0xFA] = unofficial_nop(AddressingMode::Implied, 1, 2);
    table[0xFB] = unofficial("*ISC", AddressingMode::AbsoluteY, 3, 7);
    table[0xFC] = unofficial_nop(AddressingMode::AbsoluteX, 3, 4);
    table[0xFD] = opcode("SBC", AddressingMode::AbsoluteX, 3, 4, true);
    table[0xFE] = opcode("INC", AddressingMode::AbsoluteX, 3, 7, true);
    table[0xFF] = unofficial("*ISC", AddressingMode::AbsoluteX, 3, 7);

    table
}

pub const OPCODES: [OpcodeMeta; 256] = build_table();

pub fn opcode_meta(opcode: u8) -> &'static OpcodeMeta {
    &OPCODES[opcode as usize]
}
