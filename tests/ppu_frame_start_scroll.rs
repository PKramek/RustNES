mod support;

use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::console::Console;
use RustNES::core::ppu::STATUS_SPRITE_ZERO_HIT;

use support::cartridge_from_program;

fn chr_ram_cartridge() -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 0, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    rom.extend(std::iter::repeat_n(0xEA, 0x4000));
    load_cartridge_from_bytes(&rom).expect("CHR-RAM fixture should build")
}

fn chr_ram_console() -> Console {
    let mut console = Console::new(cartridge_from_program(
        &[(0xC000, 0xEA)],
        0xC000,
        0xC000,
        0xC000,
    ));
    console.reset();
    console
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
    bus.refresh_ppu_framebuffer();
}

fn advance_to_completed_frame(bus: &mut Bus) {
    advance_cpu_ticks(bus, (241 * 341 + 1) / 3 + 1);
    assert!(bus.ppu_mut().take_frame_ready());
    advance_cpu_ticks(bus, (262 * 341) / 3 + 1);
}

fn seed_tiles_for_frame_start_seam(bus: &mut Bus) {
    write_ppu(bus, 0x0010, &[0xFF; 8]);
    write_ppu(bus, 0x0018, &[0x00; 8]);
    write_ppu(bus, 0x0020, &[0x00; 8]);
    write_ppu(bus, 0x0028, &[0xFF; 8]);
    write_ppu(bus, 0x0030, &[0xFF; 8]);
    write_ppu(bus, 0x0038, &[0xFF; 8]);
    write_ppu(bus, 0x2000, &[0x01, 0x02]);
    write_ppu(bus, 0x2800, &[0x02, 0x01]);
    write_ppu(bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(bus, 0x3F10, &[0x0F, 0x2A, 0x2B, 0x2C]);
}

fn seed_sprite_zero_frame_start_seam(bus: &mut Bus) {
    write_ppu(bus, 0x0010, &[0xFF; 8]);
    write_ppu(bus, 0x0018, &[0x00; 8]);
    write_ppu(bus, 0x0020, &[0xFF; 8]);
    write_ppu(bus, 0x0028, &[0xFF; 8]);
    write_ppu(bus, 0x2000 + 32 + 1, &[0x01]);
    write_ppu(bus, 0x2800 + 32 + 1, &[0x02]);
    write_ppu(bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    write_ppu(bus, 0x3F10, &[0x0F, 0x2A, 0x2B, 0x2C]);
}

fn force_frame_start_mismatch(bus: &mut Bus) {
    bus.write(0x2000, 0x01);
    bus.write(0x2006, 0x00);
    bus.write(0x2006, 0x00);
    bus.write(0x2005, 0x00);
    bus.write(0x2005, 0x00);
}

#[test]
fn frame_start_background_uses_latched_scroll_state_not_ctrl_bits() {
    let mut bus = Bus::new(chr_ram_cartridge());
    seed_tiles_for_frame_start_seam(&mut bus);
    force_frame_start_mismatch(&mut bus);
    bus.write(0x2001, 0x08);

    advance_to_completed_frame(&mut bus);

    let framebuffer = bus.ppu().framebuffer();
    assert_eq!(framebuffer[0], 0x11);
    assert_eq!(framebuffer[8], 0x22);
}

#[test]
fn frame_start_sprite_zero_hit_uses_latched_scroll_state() {
    let mut bus = Bus::new(chr_ram_cartridge());
    seed_sprite_zero_frame_start_seam(&mut bus);
    force_frame_start_mismatch(&mut bus);

    bus.write(0x2001, 0x18);
    bus.write(0x2003, 0x00);
    bus.write(0x2004, 0x07);
    bus.write(0x2004, 0x02);
    bus.write(0x2004, 0x00);
    bus.write(0x2004, 0x08);

    advance_to_completed_frame(&mut bus);

    assert_ne!(bus.ppu().status() & STATUS_SPRITE_ZERO_HIT, 0);
    assert_eq!(bus.ppu().framebuffer()[8 * 256 + 8], 0x2C);
}

#[test]
fn split_scroll_frame_survives_setup_time_ppuaddr_traffic() {
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

    force_frame_start_mismatch(&mut bus);
    bus.write(0x2001, 0x18);

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
fn console_frame_advance_keeps_latched_frame_start_scene_stable() {
    let mut console = chr_ram_console();
    let bus = console.bus_mut();

    seed_tiles_for_frame_start_seam(bus);
    force_frame_start_mismatch(bus);
    bus.write(0x2001, 0x08);

    advance_cpu_ticks(bus, (241 * 341 + 1) / 3 + 1);
    let first_frame = bus.ppu().framebuffer().to_vec();
    assert!(bus.ppu_mut().take_frame_ready());

    let advanced = console
        .run_until_next_frame(20_000)
        .expect("NOP program should advance to the next frame");

    assert!(advanced);
    assert_eq!(console.bus().ppu().framebuffer().to_vec(), first_frame);
}
