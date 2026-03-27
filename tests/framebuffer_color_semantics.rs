use std::collections::BTreeSet;

use RustNES::core::ppu::{FRAMEBUFFER_LEN, palette_rgba, write_rgba_frame};

fn unique_rgba_colors(framebuffer: &[u8; FRAMEBUFFER_LEN]) -> BTreeSet<[u8; 4]> {
    let mut rgba = vec![0u8; FRAMEBUFFER_LEN * 4];
    write_rgba_frame(framebuffer, &mut rgba);
    rgba.chunks_exact(4)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
        .collect()
}

#[test]
fn palette_index_zero_maps_to_a_visible_rgba_color() {
    assert_eq!(palette_rgba(0x00), [0x75, 0x75, 0x75, 0xFF]);
}

#[test]
fn uniform_framebuffer_converts_each_palette_index_to_its_rgba_color() {
    let framebuffer = [0x38; FRAMEBUFFER_LEN];
    let unique_indices = framebuffer.iter().copied().collect::<BTreeSet<_>>();
    let unique_rgba = unique_rgba_colors(&framebuffer);

    assert_eq!(unique_indices, BTreeSet::from([0x38]));
    assert_eq!(unique_rgba, BTreeSet::from([palette_rgba(0x38)]));
}

#[test]
fn zero_palette_entries_stay_visible_in_rgba_output() {
    let mut framebuffer = [0x00; FRAMEBUFFER_LEN];
    for (index, pixel) in framebuffer.iter_mut().enumerate() {
        if index % 2 == 1 {
            *pixel = 0x3F;
        }
    }

    let unique_indices = framebuffer.iter().copied().collect::<BTreeSet<_>>();
    let unique_rgba = unique_rgba_colors(&framebuffer);

    assert_eq!(unique_indices, BTreeSet::from([0x00, 0x3F]));
    assert_eq!(
        unique_rgba,
        BTreeSet::from([palette_rgba(0x00), palette_rgba(0x3F)])
    );
}
