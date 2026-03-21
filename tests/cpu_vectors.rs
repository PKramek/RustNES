use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::bus::CpuBus;
use RustNES::core::console::Console;

fn cartridge_with_vectors(reset: u16, nmi: u16, irq: u16) -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 2, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    let mut prg = vec![0x00; 0x8000];
    prg[0x7FFA] = (nmi & 0x00FF) as u8;
    prg[0x7FFB] = (nmi >> 8) as u8;
    prg[0x7FFC] = (reset & 0x00FF) as u8;
    prg[0x7FFD] = (reset >> 8) as u8;
    prg[0x7FFE] = (irq & 0x00FF) as u8;
    prg[0x7FFF] = (irq >> 8) as u8;
    rom.extend(prg);
    rom.extend(std::iter::repeat_n(0x00, 0x2000));
    load_cartridge_from_bytes(&rom).expect("fixture cartridge should build")
}

#[test]
fn reset_vector_contract_is_exposed_through_console() {
    let mut console = Console::new(cartridge_with_vectors(0xC123, 0xD234, 0xE345));
    console.reset();
    assert_eq!(console.cpu().pc, 0xC123);
    assert_eq!(console.cpu().sp, 0xFD);
    assert_eq!(console.cpu().status, 0x24);
}

#[test]
fn brk_pushes_return_address_and_vectors_through_irq() {
    let mut console = Console::new(cartridge_with_vectors(0xC000, 0xD234, 0xE345));
    console.reset();

    let record = console.step_instruction().expect("BRK should be implemented");

    assert_eq!(record.pc_before, 0xC000);
    assert_eq!(console.cpu().pc, 0xE345);
    assert_eq!(console.cpu().sp, 0xFA);
    assert_eq!(console.bus_mut().read(0x01FD), 0xC0);
    assert_eq!(console.bus_mut().read(0x01FC), 0x02);
    assert_eq!(console.bus_mut().read(0x01FB), 0x34);
}

#[test]
fn rti_restores_status_and_program_counter() {
    let mut console = Console::new(cartridge_with_vectors(0xC000, 0xD234, 0xE345));
    console.reset();
    console.service_brk();
    console.return_from_interrupt();

    assert_eq!(console.cpu().pc, 0xC002);
    assert_eq!(console.cpu().sp, 0xFD);
    assert_eq!(console.cpu().status, 0x24);
}

#[test]
fn nmi_service_uses_nmi_vector_and_clears_break_flag_on_stack() {
    let mut console = Console::new(cartridge_with_vectors(0xC000, 0xD234, 0xE345));
    console.reset();
    console.cpu_mut().status = 0x20;
    console.service_nmi();

    assert_eq!(console.cpu().pc, 0xD234);
    assert_eq!(console.bus_mut().read(0x01FB), 0x20);
}