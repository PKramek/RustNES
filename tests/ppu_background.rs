mod support;

use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::console::Console;

use support::cartridge_from_program;

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
fn background_renderer_composes_tiles_and_palettes_into_framebuffer() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0x00; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000, &[0x00; 0x400]);
    write_ppu(&mut bus, 0x2000, &[0x01, 0x02]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    bus.write(0x2000, 0x00);
    bus.write(0x2005, 0x00);
    bus.write(0x2005, 0x00);
    bus.write(0x2001, 0x08);

    advance_to_vblank(&mut bus);

    let framebuffer = bus.ppu().framebuffer();
    assert_eq!(&framebuffer[0..8], &[0x11; 8]);
    assert_eq!(&framebuffer[8..16], &[0x22; 8]);
    assert!(bus.ppu().frame_ready());
}

#[test]
fn scroll_writes_shift_the_rendered_viewport() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0x00; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000, &[0x01, 0x02]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);
    bus.write(0x2000, 0x00);
    bus.write(0x2001, 0x08);
    bus.write(0x2005, 0x08);
    bus.write(0x2005, 0x00);

    advance_to_vblank(&mut bus);

    let framebuffer = bus.ppu().framebuffer();
    assert_eq!(&framebuffer[0..8], &[0x22; 8]);
}

#[test]
fn mid_scanline_scroll_writes_only_affect_pixels_after_the_write_dot() {
    let mut bus = Bus::new(chr_ram_cartridge());

    write_ppu(&mut bus, 0x0010, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0018, &[0x00; 8]);
    write_ppu(&mut bus, 0x0020, &[0x00; 8]);
    write_ppu(&mut bus, 0x0028, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0030, &[0xFF; 8]);
    write_ppu(&mut bus, 0x0038, &[0xFF; 8]);
    write_ppu(&mut bus, 0x2000, &[0x01, 0x02, 0x03]);
    write_ppu(&mut bus, 0x3F00, &[0x0F, 0x11, 0x22, 0x33]);

    bus.write(0x2000, 0x00);
    bus.write(0x2001, 0x08);
    bus.write(0x2005, 0x00);
    bus.write(0x2005, 0x00);

    advance_to_vblank(&mut bus);
    assert!(bus.ppu_mut().take_frame_ready());

    let frame_before_split = bus.ppu().frame();
    let mut ticks_to_next_frame = 0usize;
    while bus.ppu().frame() == frame_before_split {
        bus.tick();
        ticks_to_next_frame += 1;
        assert!(
            ticks_to_next_frame < 10_000,
            "expected to reach the next frame"
        );
    }

    for _ in 0..3 {
        bus.tick();
    }

    bus.write(0x2005, 0x08);
    bus.write(0x2005, 0x00);

    let mut ticks = 0usize;
    while !bus.ppu().frame_ready() {
        bus.tick();
        ticks += 1;
        assert!(ticks < 30_000, "frame should reach vblank");
    }

    let framebuffer = bus.ppu().framebuffer();
    assert_eq!(framebuffer[0], 0x11);
    assert_eq!(framebuffer[7], 0x11);
    assert_eq!(framebuffer[12], 0x33);
}

#[test]
fn console_can_run_until_the_next_completed_frame() {
    let mut console = Console::new(cartridge_from_program(
        &[(0xC000, 0xEA)],
        0xC000,
        0xC000,
        0xC000,
    ));
    console.reset();
    console.bus_mut().write(0x2001, 0x08);

    let advanced = console
        .run_until_next_frame(20_000)
        .expect("NOP program should advance to the next frame");

    assert!(advanced);
    assert_eq!(console.bus().ppu().frame(), 1);
    assert!(console.take_frame_ready());
    assert!(!console.take_frame_ready());
}
