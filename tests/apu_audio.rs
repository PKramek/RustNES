use RustNES::core::apu::FrameCounterMode;
use RustNES::core::bus::{Bus, CpuBus};
use RustNES::core::cartridge::load_cartridge_from_bytes;
use RustNES::core::console::Console;

fn mapper0_cartridge() -> RustNES::core::cartridge::Cartridge {
    let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 1, 0, 0];
    rom.extend_from_slice(&[0; 8]);
    rom.extend(std::iter::repeat_n(0xAA, 0x4000));
    rom.extend(std::iter::repeat_n(0xBB, 0x2000));
    load_cartridge_from_bytes(&rom).expect("fixture cartridge should build")
}

fn configure_pulse_channel(console: &mut Console) {
    let bus = console.bus_mut();
    bus.write(0x4015, 0x01);
    bus.write(0x4000, 0b0100_1111);
    bus.write(0x4002, 0xF7);
    bus.write(0x4003, 0x08);
}

#[test]
fn controller_strobe_stays_on_4016_and_frame_counter_moves_to_4017() {
    let mut bus = Bus::new(mapper0_cartridge());

    bus.controller1_mut().set_buttons(0b0000_0001);
    bus.controller2_mut().set_buttons(0b0000_0010);

    bus.write(0x4016, 1);
    assert!(bus.controller1().strobe_high());
    assert!(bus.controller2().strobe_high());

    bus.write(0x4016, 0);
    assert!(!bus.controller1().strobe_high());
    assert!(!bus.controller2().strobe_high());
    assert_eq!(bus.read(0x4016) & 1, 1);
    assert_eq!(bus.read(0x4017) & 1, 0);
    assert_eq!(bus.controller2().shift_register(), 0b1000_0001);

    assert_eq!(bus.apu().frame_counter_mode(), FrameCounterMode::FourStep);
    bus.write(0x4017, 0x80);
    assert_eq!(bus.apu().frame_counter_mode(), FrameCounterMode::FiveStep);
    assert!(!bus.controller2().strobe_high());
}

#[test]
fn pulse_output_produces_deterministic_non_silent_samples() {
    let mut console = Console::new(mapper0_cartridge());
    configure_pulse_channel(&mut console);

    for _ in 0..20_000 {
        console.bus_mut().tick();
    }

    assert_eq!(console.audio_sample_rate(), 44_100);
    let samples = console.take_audio_samples();
    assert!(!samples.is_empty());
    assert!(samples.iter().any(|sample| sample.abs() > 0.01));
    assert!(console.take_audio_samples().is_empty());

    let mut replay = Console::new(mapper0_cartridge());
    configure_pulse_channel(&mut replay);
    for _ in 0..20_000 {
        replay.bus_mut().tick();
    }

    assert_eq!(samples, replay.take_audio_samples());
}
