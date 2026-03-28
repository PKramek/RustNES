mod support;

use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::ppu::{Ppu, STATUS_VBLANK};

use support::console_from_program;

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

#[test]
fn odd_frames_skip_one_dot_when_rendering_is_enabled() {
    let mut cartridge = mapper0_cartridge();
    let mut ppu = Ppu::default();
    ppu.cpu_write_register(0x2001, 0x08, &mut cartridge);

    while ppu.frame() == 0 {
        ppu.tick(&cartridge);
    }
    let frame_one_start = ppu.total_cycles();

    while ppu.frame() == 1 {
        ppu.tick(&cartridge);
    }
    let frame_two_start = ppu.total_cycles();

    assert_eq!(frame_two_start - frame_one_start, 262 * 341 - 1);
}

#[test]
fn even_frames_keep_full_length_when_rendering_is_disabled() {
    let cartridge = mapper0_cartridge();
    let mut ppu = Ppu::default();

    while ppu.frame() == 0 {
        ppu.tick(&cartridge);
    }
    let frame_one_start = ppu.total_cycles();

    while ppu.frame() == 1 {
        ppu.tick(&cartridge);
    }
    let frame_two_start = ppu.total_cycles();

    assert_eq!(frame_two_start - frame_one_start, 262 * 341);
}

#[test]
fn ppustatus_read_occurs_on_expected_bus_phase() {
    let mut console =
        console_from_program(&[(0x8000, 0xAD), (0x8001, 0x02), (0x8002, 0x20)], 0x8000);

    advance_cpu_ticks(console.bus_mut(), (260 * 341 + 338) / 3);
    assert_eq!(console.bus().ppu().scanline(), 260);
    assert_eq!(console.bus().ppu().dot(), 338);

    console.bus_mut().ppu_mut().set_status(STATUS_VBLANK);

    let record = console
        .step_instruction()
        .expect("LDA $2002 should execute");

    assert_eq!(record.operand_addr, Some(0x2002));
    assert_eq!(record.cycles_used(console.cpu().total_cycles), 4);
    assert_ne!(console.cpu().a & STATUS_VBLANK, 0);
    assert_eq!(console.bus().ppu().status() & STATUS_VBLANK, 0);
}
