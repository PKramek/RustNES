use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::ppu::STATUS_SPRITE_ZERO_HIT;

fn chr_ram_cartridge() -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 0, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    rom.extend(std::iter::repeat_n(0xEA, 0x4000));
    load_cartridge_from_bytes(&rom).expect("CHR-RAM fixture should build")
}

fn write_ppu(bus: &mut Bus, addr: u16, bytes: &[u8]) {
    bus.write(0x2006, (addr >> 8) as u8);
    bus.write(0x2006, addr as u8);
    for byte in bytes {
        bus.write(0x2007, *byte);
    }
}

fn advance_to_vblank(bus: &mut Bus) {
    for _ in 0..((241 * 341 + 1) / 3) {
        bus.tick();
    }
    bus.refresh_ppu_framebuffer();
}

#[test]
fn dma_copies_cpu_page_into_oam() {
    let mut bus = Bus::new(chr_ram_cartridge());

    bus.write(0x0200, 0x07);
    bus.write(0x0201, 0x02);
    bus.write(0x0202, 0x20);
    bus.write(0x0203, 0x08);
    bus.write(0x4014, 0x02);

    assert_eq!(bus.ppu().peek_oam(0x00), 0x07);
    assert_eq!(bus.ppu().peek_oam(0x01), 0x02);
    assert_eq!(bus.ppu().peek_oam(0x02), 0x20);
    assert_eq!(bus.ppu().peek_oam(0x03), 0x08);
}

#[test]
fn sprite_zero_hit_is_set_when_sprite_overlaps_opaque_background() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000 + 32 + 1, &[0x01]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(&mut bus, 0x3F10, &[0x0F, 0x2A, 0x2B, 0x2C]);

    bus.write(0x2001, 0x18);
    bus.write(0x2003, 0x00);
    bus.write(0x2004, 0x07);
    bus.write(0x2004, 0x02);
    bus.write(0x2004, 0x00);
    bus.write(0x2004, 0x08);

    advance_to_vblank(&mut bus);

    assert_ne!(bus.ppu().status() & STATUS_SPRITE_ZERO_HIT, 0);
    assert_eq!(bus.ppu().framebuffer()[8 * 256 + 8], 0x2C);
}

#[test]
fn background_priority_keeps_background_visible_but_still_sets_sprite_zero_hit() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000 + 32 + 1, &[0x01]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(&mut bus, 0x3F10, &[0x0F, 0x2A, 0x2B, 0x2C]);

    bus.write(0x2001, 0x18);
    bus.write(0x2003, 0x00);
    bus.write(0x2004, 0x07);
    bus.write(0x2004, 0x02);
    bus.write(0x2004, 0x20);
    bus.write(0x2004, 0x08);

    advance_to_vblank(&mut bus);

    assert_ne!(bus.ppu().status() & STATUS_SPRITE_ZERO_HIT, 0);
    assert_eq!(bus.ppu().framebuffer()[8 * 256 + 8], 0x11);
}

#[test]
fn lower_oam_index_sprite_wins_when_sprites_overlap() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0028, &[0x00; 8]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(
        &mut bus,
        0x3F10,
        &[0x0F, 0x2A, 0x2B, 0x2C, 0x0F, 0x16, 0x17, 0x18],
    );

    bus.write(0x2001, 0x10);
    bus.write(0x2003, 0x00);

    bus.write(0x2004, 0x07);
    bus.write(0x2004, 0x01);
    bus.write(0x2004, 0x00);
    bus.write(0x2004, 0x08);

    bus.write(0x2004, 0x07);
    bus.write(0x2004, 0x02);
    bus.write(0x2004, 0x01);
    bus.write(0x2004, 0x08);

    advance_to_vblank(&mut bus);

    assert_eq!(bus.ppu().framebuffer()[8 * 256 + 8], 0x2A);
}

#[test]
fn sprite_zero_hit_sets_during_visible_scanlines_before_vblank() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000 + 32 + 1, &[0x01]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(&mut bus, 0x3F10, &[0x0F, 0x2A, 0x2B, 0x2C]);

    bus.write(0x2001, 0x18);
    bus.write(0x2003, 0x00);
    bus.write(0x2004, 0x07);
    bus.write(0x2004, 0x02);
    bus.write(0x2004, 0x00);
    bus.write(0x2004, 0x08);

    for _ in 0..((9 * 341 + 10) / 3) {
        bus.tick();
    }

    assert_ne!(bus.ppu().status() & STATUS_SPRITE_ZERO_HIT, 0);
    assert_eq!(bus.ppu().status() & 0x80, 0);
}
