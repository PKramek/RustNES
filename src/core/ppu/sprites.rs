use crate::core::cartridge::Cartridge;

use super::{FRAMEBUFFER_LEN, PpuMemory, PpuRegisters, SCREEN_HEIGHT, SCREEN_WIDTH};

const MASK_SHOW_SPRITES: u8 = 0x10;
const CTRL_SPRITE_PATTERN_TABLE: u8 = 0x08;
const ATTR_PRIORITY_BEHIND_BACKGROUND: u8 = 0x20;
const ATTR_FLIP_HORIZONTAL: u8 = 0x40;
const ATTR_FLIP_VERTICAL: u8 = 0x80;

pub fn compose_sprites(
    memory: &PpuMemory,
    registers: &PpuRegisters,
    framebuffer: &mut [u8; FRAMEBUFFER_LEN],
    background_opaque: &[bool; FRAMEBUFFER_LEN],
    cartridge: &Cartridge,
) -> bool {
    if registers.mask & MASK_SHOW_SPRITES == 0 {
        return false;
    }

    let pattern_base = if registers.ctrl & CTRL_SPRITE_PATTERN_TABLE != 0 {
        0x1000
    } else {
        0x0000
    };

    let mut sprite_zero_hit = false;

    for sprite_index in 0..64 {
        let base = (sprite_index * 4) as u8;
        let y = memory.oam_read(base) as usize + 1;
        let tile = memory.oam_read(base.wrapping_add(1));
        let attr = memory.oam_read(base.wrapping_add(2));
        let x = memory.oam_read(base.wrapping_add(3)) as usize;

        for row in 0..8 {
            let screen_y = y + row;
            if screen_y >= SCREEN_HEIGHT {
                continue;
            }

            let pattern_row = if attr & ATTR_FLIP_VERTICAL != 0 { 7 - row } else { row };
            let pattern_addr = pattern_base + (tile as u16) * 16 + pattern_row as u16;
            let low_plane = memory.peek(pattern_addr, cartridge);
            let high_plane = memory.peek(pattern_addr + 8, cartridge);

            for column in 0..8 {
                let screen_x = x + column;
                if screen_x >= SCREEN_WIDTH {
                    continue;
                }

                let fine_x = if attr & ATTR_FLIP_HORIZONTAL != 0 {
                    column
                } else {
                    7 - column
                };
                let color_low = (low_plane >> fine_x) & 0x01;
                let color_high = (high_plane >> fine_x) & 0x01;
                let palette_index = (color_high << 1) | color_low;
                if palette_index == 0 {
                    continue;
                }

                let framebuffer_index = screen_y * SCREEN_WIDTH + screen_x;
                if sprite_index == 0 && background_opaque[framebuffer_index] {
                    sprite_zero_hit = true;
                }

                if attr & ATTR_PRIORITY_BEHIND_BACKGROUND != 0 && background_opaque[framebuffer_index] {
                    continue;
                }

                let palette_addr = 0x3F10 + ((attr as u16 & 0x03) * 4) + palette_index as u16;
                framebuffer[framebuffer_index] = memory.peek(palette_addr, cartridge);
            }
        }
    }

    sprite_zero_hit
}