use std::collections::BTreeSet;
use std::path::PathBuf;

use RustNES::core::cartridge::load_cartridge_from_path;
use RustNES::core::console::Console;
use RustNES::core::ppu::{FRAMEBUFFER_LEN, palette_rgba, write_rgba_frame};

fn rom_path(relative_path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn run_framebuffer(relative_path: &str, frames: usize) -> [u8; FRAMEBUFFER_LEN] {
    let cartridge =
        load_cartridge_from_path(&rom_path(relative_path)).expect("fixture ROM should load");
    let mut console = Console::new(cartridge);
    console.reset();

    for _ in 0..frames {
        assert!(
            console
                .run_until_next_frame(200_000)
                .expect("frame should advance"),
            "expected the ROM to reach the next frame"
        );
    }

    console.refresh_framebuffer();
    *console.bus().ppu().framebuffer()
}

#[test]
fn palette_index_zero_maps_to_a_visible_rgba_color() {
    assert_eq!(palette_rgba(0x00), [0x75, 0x75, 0x75, 0xFF]);
}

#[test]
fn full_palette_framebuffer_converts_to_visible_rgba_pixels() {
    let framebuffer = run_framebuffer("nestest/full_palette/full_palette.nes", 10);
    let mut rgba = vec![0u8; FRAMEBUFFER_LEN * 4];
    write_rgba_frame(&framebuffer, &mut rgba);

    let unique_indices = framebuffer.iter().copied().collect::<BTreeSet<_>>();
    let unique_rgba = rgba
        .chunks_exact(4)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect::<BTreeSet<_>>();

    assert_eq!(unique_indices, BTreeSet::from([0x38]));
    assert_eq!(unique_rgba, BTreeSet::from([palette_rgba(0x38)]));
}

#[test]
fn tvpassfail_uses_visible_background_color_even_when_raw_pixels_include_zero() {
    let framebuffer = run_framebuffer("nestest/tvpassfail/tv.nes", 5);
    let mut rgba = vec![0u8; FRAMEBUFFER_LEN * 4];
    write_rgba_frame(&framebuffer, &mut rgba);

    let unique_indices = framebuffer.iter().copied().collect::<BTreeSet<_>>();
    let unique_rgba = rgba
        .chunks_exact(4)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect::<BTreeSet<_>>();

    assert_eq!(unique_indices, BTreeSet::from([0x00, 0x3F]));
    assert_eq!(
        unique_rgba,
        BTreeSet::from([palette_rgba(0x00), palette_rgba(0x3F)])
    );
}
