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

fn advance_cpu_ticks(bus: &mut Bus, ticks: usize) {
    for _ in 0..ticks {
        bus.tick();
    }
}

#[test]
fn smb_style_hud_split_keeps_top_rows_static_while_playfield_scrolls() {
    let mut bus = Bus::new(chr_ram_cartridge());
    let mut nametable = [0u8; 0x400];
    for row in 0..30 {
        nametable[row * 32] = 0x01;
        nametable[row * 32 + 1] = 0x02;
    }

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0x00; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0030, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0038, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000, &nametable);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(&mut bus, 0x3F10, &[0x0F, 0x2A, 0x2B, 0x2C]);

    bus.write(0x2001, 0x18);
    bus.write(0x2005, 0x00);
    bus.write(0x2005, 0x00);

    bus.write(0x2003, 0x00);
    bus.write(0x2004, 39);
    bus.write(0x2004, 0x03);
    bus.write(0x2004, 0x00);
    bus.write(0x2004, 0x00);

    advance_cpu_ticks(&mut bus, (40 * 341) / 3 + 1);
    bus.write(0x2005, 0x08);
    bus.write(0x2005, 0x00);
    advance_cpu_ticks(&mut bus, ((241 - 40) * 341) / 3 + 2);

    let framebuffer = bus.ppu().framebuffer();
    assert_eq!(framebuffer[0], 0x11);
    assert_eq!(framebuffer[60 * 256], 0x22);
    assert_ne!(bus.ppu().status() & STATUS_SPRITE_ZERO_HIT, 0);
}

#[test]
fn smb_style_title_frame_is_stable_across_repeated_frames() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x2000, &[0x01; 0x400]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    bus.write(0x2001, 0x08);

    advance_cpu_ticks(&mut bus, (241 * 341 + 1) / 3 + 1);
    let first_frame = bus.ppu().framebuffer().to_vec();
    assert!(bus.ppu_mut().take_frame_ready());

    advance_cpu_ticks(&mut bus, (262 * 341) / 3 + 1);
    let second_frame = bus.ppu().framebuffer().to_vec();

    assert_eq!(first_frame, second_frame);
}
