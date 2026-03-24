mod support;

use RustNES::core::bus::CpuBus;
use RustNES::core::cpu::{
    AddressingMode, STATUS_CARRY, STATUS_NEGATIVE, STATUS_OVERFLOW, opcode_meta,
};

use support::console_from_program;

#[test]
fn zero_page_indexing_wraps_and_loads_expected_value() {
    let mut console = console_from_program(
        &[
            (0xC000, 0xA2),
            (0xC001, 0x01),
            (0xC002, 0xB5),
            (0xC003, 0xFF),
        ],
        0xC000,
    );
    console.bus_mut().write(0x0000, 0x42);

    console
        .step_instruction()
        .expect("LDX immediate should execute");
    let record = console
        .step_instruction()
        .expect("LDA zero-page,X should execute");

    assert_eq!(record.operand_addr, Some(0x0000));
    assert_eq!(console.cpu().a, 0x42);
    assert_eq!(record.cycles_used(console.cpu().total_cycles), 4);
}

#[test]
fn absolute_index_load_adds_page_cross_cycle() {
    let mut console = console_from_program(
        &[
            (0xC000, 0xA2),
            (0xC001, 0x01),
            (0xC002, 0xBD),
            (0xC003, 0xFF),
            (0xC004, 0x01),
        ],
        0xC000,
    );
    console.bus_mut().write(0x0200, 0x77);

    console
        .step_instruction()
        .expect("LDX immediate should execute");
    let record = console
        .step_instruction()
        .expect("LDA absolute,X should execute");

    assert!(record.page_crossed);
    assert_eq!(console.cpu().a, 0x77);
    assert_eq!(record.cycles_used(console.cpu().total_cycles), 5);
}

#[test]
fn branch_timing_distinguishes_taken_and_cross_page_cases() {
    let mut not_taken =
        console_from_program(&[(0xC000, 0xF0), (0xC001, 0x02), (0xC002, 0xEA)], 0xC000);
    let not_taken_record = not_taken.step_instruction().expect("BEQ should execute");
    assert_eq!(not_taken.cpu().pc, 0xC002);
    assert_eq!(
        not_taken_record.cycles_used(not_taken.cpu().total_cycles),
        2
    );

    let mut taken = console_from_program(&[(0xC0FD, 0xD0), (0xC0FE, 0x02), (0xC101, 0xEA)], 0xC0FD);
    let taken_record = taken.step_instruction().expect("BNE should execute");
    assert_eq!(taken.cpu().pc, 0xC101);
    assert!(taken_record.page_crossed);
    assert_eq!(taken_record.cycles_used(taken.cpu().total_cycles), 4);
}

#[test]
fn adc_and_sbc_update_carry_overflow_and_negative_flags() {
    let mut console = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x50),
            (0xC002, 0x18),
            (0xC003, 0x69),
            (0xC004, 0x50),
            (0xC005, 0x38),
            (0xC006, 0xE9),
            (0xC007, 0x10),
        ],
        0xC000,
    );

    console.step_instruction().expect("LDA should execute");
    console.step_instruction().expect("CLC should execute");
    console.step_instruction().expect("ADC should execute");

    assert_eq!(console.cpu().a, 0xA0);
    assert_eq!(console.cpu().status & STATUS_CARRY, 0);
    assert_ne!(console.cpu().status & STATUS_OVERFLOW, 0);
    assert_ne!(console.cpu().status & STATUS_NEGATIVE, 0);

    console.step_instruction().expect("SEC should execute");
    console.step_instruction().expect("SBC should execute");

    assert_eq!(console.cpu().a, 0x90);
    assert_ne!(console.cpu().status & STATUS_CARRY, 0);
    assert_ne!(console.cpu().status & STATUS_NEGATIVE, 0);
}

#[test]
fn jsr_and_rts_restore_control_flow() {
    let mut console = console_from_program(
        &[
            (0xC000, 0x20),
            (0xC001, 0x10),
            (0xC002, 0xC0),
            (0xC003, 0xEA),
            (0xC010, 0xE8),
            (0xC011, 0x60),
        ],
        0xC000,
    );

    let jsr_record = console.step_instruction().expect("JSR should execute");
    assert_eq!(jsr_record.cycles_used(console.cpu().total_cycles), 6);
    assert_eq!(console.cpu().pc, 0xC010);

    console.step_instruction().expect("INX should execute");
    console.step_instruction().expect("RTS should execute");

    assert_eq!(console.cpu().x, 1);
    assert_eq!(console.cpu().pc, 0xC003);
}

#[test]
fn indirect_jmp_uses_6502_page_wrap_bug() {
    let mut console =
        console_from_program(&[(0xC000, 0x6C), (0xC001, 0xFF), (0xC002, 0x10)], 0xC000);
    console.bus_mut().write(0x10FF, 0x34);
    console.bus_mut().write(0x1000, 0x12);
    console.bus_mut().write(0x1100, 0xAB);

    console
        .step_instruction()
        .expect("JMP indirect should execute");

    assert_eq!(console.cpu().pc, 0x1234);
}

#[test]
fn nestest_unofficial_nops_have_expected_metadata_and_timing() {
    let cases = [
        (0x04, AddressingMode::ZeroPage, 2, 3),
        (0x44, AddressingMode::ZeroPage, 2, 3),
        (0x64, AddressingMode::ZeroPage, 2, 3),
        (0x0C, AddressingMode::Absolute, 3, 4),
        (0x14, AddressingMode::ZeroPageX, 2, 4),
        (0x34, AddressingMode::ZeroPageX, 2, 4),
        (0x54, AddressingMode::ZeroPageX, 2, 4),
        (0x74, AddressingMode::ZeroPageX, 2, 4),
        (0xD4, AddressingMode::ZeroPageX, 2, 4),
        (0xF4, AddressingMode::ZeroPageX, 2, 4),
        (0x80, AddressingMode::Immediate, 2, 2),
        (0x1A, AddressingMode::Implied, 1, 2),
        (0x3A, AddressingMode::Implied, 1, 2),
        (0x5A, AddressingMode::Implied, 1, 2),
        (0x7A, AddressingMode::Implied, 1, 2),
        (0xDA, AddressingMode::Implied, 1, 2),
        (0xFA, AddressingMode::Implied, 1, 2),
        (0x1C, AddressingMode::AbsoluteX, 3, 4),
        (0x3C, AddressingMode::AbsoluteX, 3, 4),
        (0x5C, AddressingMode::AbsoluteX, 3, 4),
        (0x7C, AddressingMode::AbsoluteX, 3, 4),
        (0xDC, AddressingMode::AbsoluteX, 3, 4),
        (0xFC, AddressingMode::AbsoluteX, 3, 4),
    ];

    for (opcode, mode, bytes, cycles) in cases {
        let meta = opcode_meta(opcode);
        assert_eq!(meta.mnemonic, "*NOP");
        assert_eq!(meta.mode, mode);
        assert_eq!(meta.bytes, bytes);
        assert_eq!(meta.base_cycles, cycles);
        assert!(!meta.official);
    }

    let mut zero_page = console_from_program(&[(0xC000, 0x04), (0xC001, 0x10)], 0xC000);
    zero_page.bus_mut().write(0x0010, 0x33);
    let before = *zero_page.cpu();
    let record = zero_page
        .step_instruction()
        .expect("unofficial zero-page NOP should execute");
    assert_eq!(record.operand_addr, Some(0x0010));
    assert_eq!(record.operand_value, Some(0x33));
    assert_eq!(record.cycles_used(zero_page.cpu().total_cycles), 3);
    assert_eq!(zero_page.cpu().a, before.a);
    assert_eq!(zero_page.cpu().x, before.x);
    assert_eq!(zero_page.cpu().y, before.y);
    assert_eq!(zero_page.cpu().status, before.status);

    let mut zero_page_x = console_from_program(
        &[
            (0xC000, 0xA2),
            (0xC001, 0x01),
            (0xC002, 0x14),
            (0xC003, 0xFF),
        ],
        0xC000,
    );
    zero_page_x.bus_mut().write(0x0000, 0x44);
    zero_page_x
        .step_instruction()
        .expect("LDX immediate should execute");
    let record = zero_page_x
        .step_instruction()
        .expect("unofficial zero-page,X NOP should execute");
    assert_eq!(record.operand_addr, Some(0x0000));
    assert_eq!(record.operand_value, Some(0x44));
    assert_eq!(record.cycles_used(zero_page_x.cpu().total_cycles), 4);

    let mut absolute_x = console_from_program(
        &[
            (0xC000, 0xA2),
            (0xC001, 0x01),
            (0xC002, 0x1C),
            (0xC003, 0xFF),
            (0xC004, 0x01),
        ],
        0xC000,
    );
    absolute_x.bus_mut().write(0x0200, 0x55);
    absolute_x
        .step_instruction()
        .expect("LDX immediate should execute");
    let record = absolute_x
        .step_instruction()
        .expect("unofficial absolute,X NOP should execute");
    assert!(record.page_crossed);
    assert_eq!(record.operand_addr, Some(0x0200));
    assert_eq!(record.operand_value, Some(0x55));
    assert_eq!(record.cycles_used(absolute_x.cpu().total_cycles), 5);

    let mut immediate = console_from_program(&[(0xC000, 0x80), (0xC001, 0x89)], 0xC000);
    let record = immediate
        .step_instruction()
        .expect("unofficial immediate NOP should execute");
    assert_eq!(record.operand_value, Some(0x89));
    assert_eq!(record.cycles_used(immediate.cpu().total_cycles), 2);

    let mut implied = console_from_program(&[(0xC000, 0x1A)], 0xC000);
    let record = implied
        .step_instruction()
        .expect("unofficial implied NOP should execute");
    assert_eq!(record.cycles_used(implied.cpu().total_cycles), 2);
}

#[test]
fn unofficial_alu_and_load_store_opcodes_match_expected_effects() {
    let mut slo = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x01),
            (0xC002, 0x07),
            (0xC003, 0x10),
        ],
        0xC000,
    );
    slo.bus_mut().write(0x0010, 0x81);
    slo.step_instruction().expect("LDA should execute");
    let record = slo.step_instruction().expect("SLO should execute");
    assert_eq!(record.cycles_used(slo.cpu().total_cycles), 5);
    assert_eq!(slo.bus_mut().read(0x0010), 0x02);
    assert_eq!(slo.cpu().a, 0x03);
    assert_ne!(slo.cpu().status & STATUS_CARRY, 0);

    let mut rla = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0xFF),
            (0xC002, 0x18),
            (0xC003, 0x27),
            (0xC004, 0x10),
        ],
        0xC000,
    );
    rla.bus_mut().write(0x0010, 0x80);
    rla.step_instruction().expect("LDA should execute");
    rla.step_instruction().expect("CLC should execute");
    rla.step_instruction().expect("RLA should execute");
    assert_eq!(rla.bus_mut().read(0x0010), 0x00);
    assert_eq!(rla.cpu().a, 0x00);
    assert_ne!(rla.cpu().status & STATUS_CARRY, 0);

    let mut sre = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x03),
            (0xC002, 0x47),
            (0xC003, 0x10),
        ],
        0xC000,
    );
    sre.bus_mut().write(0x0010, 0x02);
    sre.step_instruction().expect("LDA should execute");
    sre.step_instruction().expect("SRE should execute");
    assert_eq!(sre.bus_mut().read(0x0010), 0x01);
    assert_eq!(sre.cpu().a, 0x02);

    let mut rra = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x40),
            (0xC002, 0x18),
            (0xC003, 0x67),
            (0xC004, 0x10),
        ],
        0xC000,
    );
    rra.bus_mut().write(0x0010, 0x02);
    rra.step_instruction().expect("LDA should execute");
    rra.step_instruction().expect("CLC should execute");
    rra.step_instruction().expect("RRA should execute");
    assert_eq!(rra.bus_mut().read(0x0010), 0x01);
    assert_eq!(rra.cpu().a, 0x41);

    let mut sax = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0xCC),
            (0xC002, 0xA2),
            (0xC003, 0xAA),
            (0xC004, 0x87),
            (0xC005, 0x10),
        ],
        0xC000,
    );
    sax.step_instruction().expect("LDA should execute");
    sax.step_instruction().expect("LDX should execute");
    sax.step_instruction().expect("SAX should execute");
    assert_eq!(sax.bus_mut().read(0x0010), 0x88);

    let mut lax = console_from_program(&[(0xC000, 0xA7), (0xC001, 0x10)], 0xC000);
    lax.bus_mut().write(0x0010, 0x55);
    lax.step_instruction().expect("LAX should execute");
    assert_eq!(lax.cpu().a, 0x55);
    assert_eq!(lax.cpu().x, 0x55);

    let mut dcp = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x10),
            (0xC002, 0xC7),
            (0xC003, 0x10),
        ],
        0xC000,
    );
    dcp.bus_mut().write(0x0010, 0x10);
    dcp.step_instruction().expect("LDA should execute");
    dcp.step_instruction().expect("DCP should execute");
    assert_eq!(dcp.bus_mut().read(0x0010), 0x0F);
    assert_ne!(dcp.cpu().status & STATUS_CARRY, 0);

    let mut isc = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x10),
            (0xC002, 0x38),
            (0xC003, 0xE7),
            (0xC004, 0x10),
        ],
        0xC000,
    );
    isc.bus_mut().write(0x0010, 0x01);
    isc.step_instruction().expect("LDA should execute");
    isc.step_instruction().expect("SEC should execute");
    isc.step_instruction().expect("ISC should execute");
    assert_eq!(isc.bus_mut().read(0x0010), 0x02);
    assert_eq!(isc.cpu().a, 0x0E);

    let mut unofficial_sbc = console_from_program(
        &[
            (0xC000, 0xA9),
            (0xC001, 0x10),
            (0xC002, 0x38),
            (0xC003, 0xEB),
            (0xC004, 0x01),
        ],
        0xC000,
    );
    unofficial_sbc
        .step_instruction()
        .expect("LDA should execute");
    unofficial_sbc
        .step_instruction()
        .expect("SEC should execute");
    unofficial_sbc
        .step_instruction()
        .expect("unofficial SBC should execute");
    assert_eq!(unofficial_sbc.cpu().a, 0x0F);
}
