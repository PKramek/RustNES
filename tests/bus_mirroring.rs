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
fn bus_contracts_compile_for_ram_and_register_mirrors() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x0000, 0x12);
    assert_eq!(bus.read(0x0800), 0x12);
    assert_eq!(Bus::normalize_ppu_register_addr(0x2008), 0x2000);
    bus.write(0x4014, 0x44);
    assert_eq!(bus.dma().last_page(), Some(0x44));
}