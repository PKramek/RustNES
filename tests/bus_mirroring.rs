use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;

fn mapper0_cartridge() -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    rom.extend(std::iter::repeat_n(0xAA, 0x4000));
    rom.extend(std::iter::repeat_n(0xBB, 0x2000));
    load_cartridge_from_bytes(&rom).expect("fixture cartridge should build")
}

#[test]
fn ram_mirrors_share_one_backing_array() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x0000, 0x12);
    assert_eq!(bus.read(0x0800), 0x12);
    bus.write(0x17FF, 0x34);
    assert_eq!(bus.read(0x07FF), 0x34);
}

#[test]
fn ppu_register_mirrors_normalize_and_status_read_clears_flags() {
    let mut bus = Bus::new(mapper0_cartridge());

    assert_eq!(Bus::normalize_ppu_register_addr(0x2008), 0x2000);
    assert_eq!(Bus::normalize_ppu_register_addr(0x3FFF), 0x2007);

    bus.write(0x2008, 0x81);
    assert_eq!(bus.ppu().ctrl(), 0x81);

    bus.ppu_mut().set_status(0xE0);
    bus.write(0x2005, 0x11);
    assert!(bus.ppu().write_toggle());
    assert_eq!(bus.read(0x2002), 0xE0);
    assert!(!bus.ppu().write_toggle());
    assert_eq!(bus.read(0x2002), 0x60);
}

#[test]
fn bus_tick_advances_ppu_three_cycles_per_cpu_cycle() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.tick();

    assert_eq!(bus.total_cpu_cycles(), 1);
    assert_eq!(bus.ppu().total_cycles(), 3);
    assert_eq!(bus.ppu().dot(), 3);
}

#[test]
fn dma_and_controller_ports_are_deterministic() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x0000, 0xAB);
    bus.write(0x4014, 0x44);
    assert_eq!(bus.dma().last_page(), Some(0x44));
    assert!(!bus.dma().pending());
    assert_eq!(bus.ppu().peek_oam(0x00), 0x00);

    bus.controller1_mut().set_buttons(0b0000_0101);
    bus.write(0x4016, 1);
    assert!(bus.controller1().strobe_high());
    assert_eq!(bus.read(0x4016) & 1, 1);

    bus.write(0x4016, 0);
    assert_eq!(bus.read(0x4016) & 1, 1);
    assert_eq!(bus.read(0x4016) & 1, 0);
    assert_eq!(bus.read(0x4016) & 1, 1);
}
