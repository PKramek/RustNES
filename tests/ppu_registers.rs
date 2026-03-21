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
fn ppuaddr_and_ppudata_follow_increment_rules() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x2006, 0x20);
    bus.write(0x2006, 0x00);
    bus.write(0x2007, 0x12);
    bus.write(0x2007, 0x34);

    assert_eq!(bus.ppu().peek_memory(0x2000, bus.cartridge()), 0x12);
    assert_eq!(bus.ppu().peek_memory(0x2001, bus.cartridge()), 0x34);
    assert_eq!(bus.ppu().vram_addr(), 0x2002);

    bus.write(0x2000, 0x04);
    bus.write(0x2006, 0x24);
    bus.write(0x2006, 0x00);
    bus.write(0x2007, 0x56);

    assert_eq!(bus.ppu().peek_memory(0x2400, bus.cartridge()), 0x56);
    assert_eq!(bus.ppu().vram_addr(), 0x2420);
}

#[test]
fn ppuscroll_and_ppuaddr_capture_scroll_latches() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x2000, 0x02);
    bus.write(0x2005, 0x2D);
    assert!(bus.ppu().write_toggle());
    assert_eq!(bus.ppu().fine_x_scroll(), 0x05);
    assert_eq!(bus.ppu().temp_vram_addr() & 0x001F, 0x05);

    bus.write(0x2005, 0xC6);
    assert!(!bus.ppu().write_toggle());
    assert_eq!(bus.ppu().temp_vram_addr() & 0x0C00, 0x0800);

    bus.write(0x2006, 0x3F);
    bus.write(0x2006, 0x10);
    assert_eq!(bus.ppu().vram_addr(), 0x3F10);
}

#[test]
fn ppudata_reads_are_buffered_and_palette_reads_bypass_buffer() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x2006, 0x20);
    bus.write(0x2006, 0x00);
    bus.write(0x2007, 0xAB);

    bus.write(0x2006, 0x20);
    bus.write(0x2006, 0x00);
    assert_eq!(bus.read(0x2007), 0x00);
    assert_eq!(bus.read(0x2007), 0xAB);

    bus.write(0x2006, 0x3F);
    bus.write(0x2006, 0x10);
    bus.write(0x2007, 0x2C);

    bus.write(0x2006, 0x3F);
    bus.write(0x2006, 0x00);
    assert_eq!(bus.read(0x2007), 0x2C);
    assert_eq!(bus.ppu().peek_memory(0x3F10, bus.cartridge()), 0x2C);
    assert_eq!(bus.ppu().peek_memory(0x3F00, bus.cartridge()), 0x2C);
}

#[test]
fn oamaddr_and_oamdata_write_into_oam() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.write(0x2003, 0x10);
    bus.write(0x2004, 0x77);
    bus.write(0x2004, 0x88);

    assert_eq!(bus.ppu().peek_oam(0x10), 0x77);
    assert_eq!(bus.ppu().peek_oam(0x11), 0x88);
    assert_eq!(bus.ppu().oam_addr(), 0x12);
}