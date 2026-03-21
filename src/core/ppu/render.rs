use crate::core::cartridge::Cartridge;

use super::{FRAMEBUFFER_LEN, PpuMemory, PpuRegisters, SCREEN_HEIGHT, SCREEN_WIDTH};

const MASK_SHOW_BACKGROUND: u8 = 0x08;
const CTRL_BACKGROUND_PATTERN_TABLE: u8 = 0x10;

pub fn render_background_frame(
    memory: &PpuMemory,
    registers: &PpuRegisters,
    scroll_x: u16,
    scroll_y: u16,
    framebuffer: &mut [u8; FRAMEBUFFER_LEN],
    cartridge: &Cartridge,
) {
    if registers.mask & MASK_SHOW_BACKGROUND == 0 {
        framebuffer.fill(0);
        return;
    }

    let base_nametable = ((registers.ctrl as u16) & 0x03) << 10;
    let scroll_x = scroll_x as usize;
    let scroll_y = scroll_y as usize;
    let pattern_base = if registers.ctrl & CTRL_BACKGROUND_PATTERN_TABLE != 0 {
        0x1000
    } else {
        0x0000
    };

    for screen_y in 0..SCREEN_HEIGHT {
        for screen_x in 0..SCREEN_WIDTH {
            let world_x = scroll_x + screen_x;
            let world_y = scroll_y + screen_y;

            let nametable_x = (world_x / SCREEN_WIDTH) & 0x01;
            let nametable_y = (world_y / SCREEN_HEIGHT) & 0x01;
            let tile_x = (world_x % SCREEN_WIDTH) / 8;
            let tile_y = (world_y % SCREEN_HEIGHT) / 8;
            let fine_x = world_x & 0x07;
            let fine_y = world_y & 0x07;

            let nametable_select = ((base_nametable >> 10) as usize + nametable_x + (nametable_y << 1)) & 0x03;
            let nametable_base = 0x2000 + (nametable_select as u16) * 0x0400;
            let nametable_addr = nametable_base + (tile_y as u16) * 32 + tile_x as u16;
            let tile_index = memory.peek(nametable_addr, cartridge);

            let attribute_addr = nametable_base + 0x03C0 + ((tile_y / 4) as u16) * 8 + (tile_x / 4) as u16;
            let attribute_byte = memory.peek(attribute_addr, cartridge);
            let attribute_shift = (((tile_y & 0x02) << 1) | (tile_x & 0x02)) as u8;
            let palette_select = (attribute_byte >> attribute_shift) & 0x03;

            let pattern_addr = pattern_base + (tile_index as u16) * 16 + fine_y as u16;
            let low_plane = memory.peek(pattern_addr, cartridge);
            let high_plane = memory.peek(pattern_addr + 8, cartridge);
            let bit = 7 - fine_x;
            let color_low = (low_plane >> bit) & 0x01;
            let color_high = (high_plane >> bit) & 0x01;
            let palette_index = (color_high << 1) | color_low;

            let color = if palette_index == 0 {
                memory.peek(0x3F00, cartridge)
            } else {
                let palette_addr = 0x3F00 + (palette_select as u16) * 4 + palette_index as u16;
                memory.peek(palette_addr, cartridge)
            };

            framebuffer[screen_y * SCREEN_WIDTH + screen_x] = color;
        }
    }
}