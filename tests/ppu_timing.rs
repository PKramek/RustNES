use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::ppu::STATUS_VBLANK;

fn mapper0_cartridge() -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    rom.extend(std::iter::repeat_n(0xAA, 0x4000));
    rom.extend(std::iter::repeat_n(0xBB, 0x2000));
    load_cartridge_from_bytes(&rom).expect("fixture cartridge should build")
}

fn advance_cpu_ticks(bus: &mut Bus, cpu_ticks: usize) {
    for _ in 0..cpu_ticks {
        bus.tick();
    }
}

#[test]
fn ppu_enters_vblank_and_raises_nmi_when_enabled() {
    let mut bus = Bus::new(mapper0_cartridge());
    bus.write(0x2000, 0x80);

    advance_cpu_ticks(&mut bus, (241 * 341 - 2) / 3);
    assert_eq!(bus.ppu().scanline(), 240);
    assert_eq!(bus.ppu().dot(), 339);
    assert_eq!(bus.ppu().status() & STATUS_VBLANK, 0);

    bus.tick();

    assert_eq!(bus.ppu().scanline(), 241);
    assert_eq!(bus.ppu().dot(), 1);
    assert_ne!(bus.ppu().status() & STATUS_VBLANK, 0);
    assert!(bus.interrupt_lines().nmi);
}

#[test]
fn reading_ppustatus_clears_vblank_and_resets_write_toggle() {
    let mut bus = Bus::new(mapper0_cartridge());
    bus.write(0x2000, 0x80);
    bus.write(0x2005, 0x12);
    assert!(bus.ppu().write_toggle());

    advance_cpu_ticks(&mut bus, (241 * 341 - 2) / 3);
    bus.tick();
    assert_ne!(bus.ppu().status() & STATUS_VBLANK, 0);

    let status = bus.read(0x2002);

    assert_ne!(status & STATUS_VBLANK, 0);
    assert_eq!(bus.ppu().status() & STATUS_VBLANK, 0);
    assert!(!bus.ppu().write_toggle());
    assert!(!bus.interrupt_lines().nmi);
}

#[test]
fn pre_render_scanline_clears_frame_flags_and_rolls_frames() {
    let mut bus = Bus::new(mapper0_cartridge());
    bus.ppu_mut().set_status(0xE0);

    advance_cpu_ticks(&mut bus, (260 * 341 + 338) / 3);
    assert_eq!(bus.ppu().scanline(), 260);
    assert_eq!(bus.ppu().dot(), 338);

    bus.tick();
    assert_eq!(bus.ppu().scanline(), 261);
    assert_eq!(bus.ppu().dot(), 0);

    bus.tick();

    assert_eq!(bus.ppu().scanline(), 261);
    assert_eq!(bus.ppu().dot(), 3);
    assert_eq!(bus.ppu().status() & 0xE0, 0);

    advance_cpu_ticks(&mut bus, 113);

    assert_eq!(bus.ppu().frame(), 1);
    assert_eq!(bus.ppu().scanline(), 0);
}

#[test]
fn frame_ready_latches_at_vblank_until_consumed() {
    let mut bus = Bus::new(mapper0_cartridge());
    assert!(!bus.ppu().frame_ready());

    advance_cpu_ticks(&mut bus, (241 * 341 - 2) / 3);
    bus.tick();

    assert!(bus.ppu().frame_ready());
    assert!(bus.ppu_mut().take_frame_ready());
    assert!(!bus.ppu().frame_ready());
}
