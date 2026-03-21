mod support;

use RustNES::core::bus::CpuBus;
use RustNES::core::cpu::{STATUS_BREAK, STATUS_CARRY, STATUS_INTERRUPT_DISABLE, STATUS_UNUSED};

use support::cartridge_from_program;

#[test]
fn php_and_plp_round_trip_ignores_break_bit_on_restore() {
    let mut console = RustNES::core::console::Console::new(cartridge_from_program(
        &[(0xC000, 0x08), (0xC001, 0x28)],
        0xC000,
        0xC000,
        0xC000,
    ));
    console.reset();
    console.cpu_mut().status = STATUS_CARRY;

    console.step_instruction().expect("PHP should execute");
    assert_eq!(console.bus_mut().read(0x01FD), STATUS_CARRY | STATUS_BREAK | STATUS_UNUSED);

    console.cpu_mut().status = 0;
    console.step_instruction().expect("PLP should execute");
    assert_eq!(console.cpu().status, STATUS_CARRY | STATUS_UNUSED);
}

#[test]
fn irq_pushes_status_without_break_and_rti_restores_program_counter() {
    let mut console = RustNES::core::console::Console::new(cartridge_from_program(
        &[(0xC000, 0xEA)],
        0xC000,
        0xD000,
        0xE000,
    ));
    console.reset();
    console.cpu_mut().pc = 0xC123;
    console.cpu_mut().status = STATUS_CARRY | STATUS_UNUSED;

    console.service_irq();

    assert_eq!(console.cpu().pc, 0xE000);
    assert_eq!(console.bus_mut().read(0x01FB), STATUS_CARRY | STATUS_UNUSED);

    console.return_from_interrupt();
    assert_eq!(console.cpu().pc, 0xC123);
    assert_eq!(console.cpu().status, STATUS_CARRY | STATUS_UNUSED);
}

#[test]
fn brk_sets_interrupt_disable_and_nmi_stack_push_clears_break_flag() {
    let mut console = RustNES::core::console::Console::new(cartridge_from_program(
        &[(0xC000, 0x00)],
        0xC000,
        0xD000,
        0xE000,
    ));
    console.reset();

    console.step_instruction().expect("BRK should execute");
    assert_ne!(console.cpu().status & STATUS_INTERRUPT_DISABLE, 0);
    assert_ne!(console.bus_mut().read(0x01FB) & STATUS_BREAK, 0);

    console.cpu_mut().pc = 0xC555;
    console.cpu_mut().status = STATUS_CARRY | STATUS_UNUSED;
    console.service_nmi();
    assert_eq!(console.bus_mut().read(0x01F8) & STATUS_BREAK, 0);
}