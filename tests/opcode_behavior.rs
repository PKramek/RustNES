mod support;

use RustNES::core::bus::CpuBus;
use RustNES::core::cpu::{STATUS_CARRY, STATUS_NEGATIVE, STATUS_OVERFLOW};

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
