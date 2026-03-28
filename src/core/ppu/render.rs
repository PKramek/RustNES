use crate::core::cartridge::Cartridge;

use super::{FRAMEBUFFER_LEN, PpuMemory, PpuRegisters, SCREEN_HEIGHT, SCREEN_WIDTH, ScrollEvent};

const MASK_SHOW_BACKGROUND: u8 = 0x08;
pub(crate) const CTRL_BACKGROUND_PATTERN_TABLE: u8 = 0x10;

pub fn render_background_frame(
    memory: &PpuMemory,
    registers: &PpuRegisters,
    scroll_events: &[ScrollEvent],
    framebuffer: &mut [u8; FRAMEBUFFER_LEN],
    background_opaque: &mut [bool; FRAMEBUFFER_LEN],
    cartridge: &Cartridge,
) {
    if registers.mask & MASK_SHOW_BACKGROUND == 0 {
        framebuffer.fill(0);
        background_opaque.fill(false);
        return;
    }

    let pattern_base = if registers.ctrl & CTRL_BACKGROUND_PATTERN_TABLE != 0 {
        0x1000
    } else {
        0x0000
    };

    for screen_y in 0..SCREEN_HEIGHT {
        for screen_x in 0..SCREEN_WIDTH {
            let (color, opaque) = background_pixel_at(
                memory,
                registers,
                scroll_events,
                screen_x,
                screen_y,
                pattern_base,
                cartridge,
            );
            let framebuffer_index = screen_y * SCREEN_WIDTH + screen_x;
            background_opaque[framebuffer_index] = opaque;
            framebuffer[framebuffer_index] = color;
        }
    }
}

pub(crate) fn background_pixel_at(
    memory: &PpuMemory,
    registers: &PpuRegisters,
    scroll_events: &[ScrollEvent],
    screen_x: usize,
    screen_y: usize,
    pattern_base: u16,
    cartridge: &Cartridge,
) -> (u8, bool) {
    if registers.mask & MASK_SHOW_BACKGROUND == 0 {
        return (0, false);
    }

    let event = scroll_event_for_pixel(scroll_events, screen_x, screen_y);
    let (base_nametable, scroll_x, scroll_y) = event_scroll_state(event);
    let world_x = scroll_x + screen_x;
    let world_y = scroll_y + screen_y;

    let nametable_x = (world_x / SCREEN_WIDTH) & 0x01;
    let nametable_y = (world_y / SCREEN_HEIGHT) & 0x01;
    let tile_x = (world_x % SCREEN_WIDTH) / 8;
    let tile_y = (world_y % SCREEN_HEIGHT) / 8;
    let fine_x = world_x & 0x07;
    let fine_y = world_y & 0x07;

    let nametable_select =
        ((base_nametable >> 10) as usize + nametable_x + (nametable_y << 1)) & 0x03;
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

    (color, palette_index != 0)
}

fn event_scroll_state(event: ScrollEvent) -> (u16, usize, usize) {
    if event.dot == 0 {
        return (
            event.temp_vram_addr & 0x0C00,
            event.scroll_x as usize,
            event.scroll_y as usize,
        );
    }

    let coarse_x = (event.temp_vram_addr & 0x001F) as usize;
    let coarse_y = ((event.temp_vram_addr >> 5) & 0x001F) as usize;
    let fine_y = ((event.temp_vram_addr >> 12) & 0x0007) as usize;
    let fine_x = (event.fine_x_scroll & 0x07) as usize;

    (
        event.base_nametable,
        coarse_x * 8 + fine_x,
        coarse_y * 8 + fine_y,
    )
}

fn scroll_event_for_pixel(
    scroll_events: &[ScrollEvent],
    screen_x: usize,
    screen_y: usize,
) -> ScrollEvent {
    let pixel_dot = screen_x as u16 + 1;
    let mut event_index = 0usize;
    while event_index + 1 < scroll_events.len() {
        let next = scroll_events[event_index + 1];
        if next.scanline < screen_y || (next.scanline == screen_y && next.dot <= pixel_dot) {
            event_index += 1;
            continue;
        }
        break;
    }
    scroll_events[event_index]
}

#[cfg(test)]
mod tests {
    use crate::core::cartridge::load_cartridge_from_bytes;

    use super::*;

    fn chr_ram_cartridge() -> Cartridge {
        let mut rom = vec![b'N', b'E', b'S', 0x1A, 1, 0, 0, 0];
        rom.extend_from_slice(&[0; 8]);
        rom.extend(std::iter::repeat_n(0xEA, 0x4000));
        load_cartridge_from_bytes(&rom).expect("CHR-RAM fixture should build")
    }

    #[test]
    fn scroll_events_apply_from_their_recorded_dot() {
        let mut cartridge = chr_ram_cartridge();
        let mut memory = PpuMemory::default();
        let registers = PpuRegisters {
            mask: MASK_SHOW_BACKGROUND,
            ..PpuRegisters::default()
        };

        memory.write(0x0010, 0xFF, &mut cartridge);
        memory.write(0x0018, 0x00, &mut cartridge);
        memory.write(0x0020, 0x00, &mut cartridge);
        memory.write(0x0028, 0xFF, &mut cartridge);
        memory.write(0x0030, 0xFF, &mut cartridge);
        memory.write(0x0038, 0xFF, &mut cartridge);
        memory.write(0x2000, 0x01, &mut cartridge);
        memory.write(0x2001, 0x02, &mut cartridge);
        memory.write(0x2002, 0x03, &mut cartridge);
        memory.write(0x3F00, 0x0F, &mut cartridge);
        memory.write(0x3F01, 0x11, &mut cartridge);
        memory.write(0x3F02, 0x22, &mut cartridge);
        memory.write(0x3F03, 0x33, &mut cartridge);

        let scroll_events = [
            ScrollEvent {
                scanline: 0,
                dot: 0,
                scroll_x: 0,
                scroll_y: 0,
                base_nametable: 0,
                fine_x_scroll: 0,
                vram_addr: 0,
                temp_vram_addr: 0,
            },
            ScrollEvent {
                scanline: 0,
                dot: 9,
                scroll_x: 8,
                scroll_y: 0,
                base_nametable: 0,
                fine_x_scroll: 0,
                vram_addr: 0x0001,
                temp_vram_addr: 0x0001,
            },
        ];

        let pattern_base = 0x0000;
        assert_eq!(
            background_pixel_at(
                &memory,
                &registers,
                &scroll_events,
                0,
                0,
                pattern_base,
                &cartridge
            ),
            (0x11, true)
        );
        assert_eq!(
            background_pixel_at(
                &memory,
                &registers,
                &scroll_events,
                7,
                0,
                pattern_base,
                &cartridge
            ),
            (0x11, true)
        );
        assert_eq!(
            background_pixel_at(
                &memory,
                &registers,
                &scroll_events,
                8,
                0,
                pattern_base,
                &cartridge
            ),
            (0x33, true)
        );
    }

    #[test]
    fn background_pixel_uses_latched_temp_vram_and_fine_x_state() {
        let mut cartridge = chr_ram_cartridge();
        let mut memory = PpuMemory::default();
        let registers = PpuRegisters {
            mask: MASK_SHOW_BACKGROUND,
            ..PpuRegisters::default()
        };

        memory.write(0x0010, 0xFF, &mut cartridge);
        memory.write(0x0018, 0x00, &mut cartridge);
        memory.write(0x0020, 0x00, &mut cartridge);
        memory.write(0x0028, 0xFF, &mut cartridge);
        memory.write(0x2000, 0x01, &mut cartridge);
        memory.write(0x2001, 0x02, &mut cartridge);
        memory.write(0x2400, 0x02, &mut cartridge);
        memory.write(0x3F00, 0x0F, &mut cartridge);
        memory.write(0x3F01, 0x11, &mut cartridge);
        memory.write(0x3F02, 0x22, &mut cartridge);

        let scroll_events = [ScrollEvent {
            scanline: 0,
            dot: 1,
            scroll_x: 0,
            scroll_y: 0,
            base_nametable: 0,
            fine_x_scroll: 3,
            vram_addr: 0x0401,
            temp_vram_addr: 0x0401,
        }];

        assert_eq!(
            background_pixel_at(
                &memory,
                &registers,
                &scroll_events,
                0,
                0,
                0x0000,
                &cartridge
            ),
            (0x22, true)
        );
    }

    #[test]
    fn visible_events_use_ctrl_nametable_when_temp_vram_was_polluted_by_ppuaddr() {
        let mut cartridge = chr_ram_cartridge();
        let mut memory = PpuMemory::default();
        let registers = PpuRegisters {
            mask: MASK_SHOW_BACKGROUND,
            ..PpuRegisters::default()
        };

        memory.write(0x0010, 0xFF, &mut cartridge);
        memory.write(0x0018, 0x00, &mut cartridge);
        memory.write(0x0020, 0x00, &mut cartridge);
        memory.write(0x0028, 0xFF, &mut cartridge);
        memory.write(0x2000, 0x01, &mut cartridge);
        memory.write(0x2001, 0x02, &mut cartridge);
        memory.write(0x3F00, 0x0F, &mut cartridge);
        memory.write(0x3F01, 0x11, &mut cartridge);
        memory.write(0x3F02, 0x22, &mut cartridge);

        let scroll_events = [ScrollEvent {
            scanline: 0,
            dot: 9,
            scroll_x: 8,
            scroll_y: 0,
            base_nametable: 0,
            fine_x_scroll: 0,
            vram_addr: 0x0001,
            temp_vram_addr: 0x0C01,
        }];

        assert_eq!(
            background_pixel_at(
                &memory,
                &registers,
                &scroll_events,
                0,
                0,
                0x0000,
                &cartridge
            ),
            (0x22, true)
        );
    }

    #[test]
    fn frame_start_events_use_latched_temp_vram_nametable_over_ctrl_bits() {
        let mut cartridge = chr_ram_cartridge();
        let mut memory = PpuMemory::default();
        let registers = PpuRegisters {
            mask: MASK_SHOW_BACKGROUND,
            ctrl: 0x01,
            ..PpuRegisters::default()
        };

        memory.write(0x0010, 0xFF, &mut cartridge);
        memory.write(0x0018, 0x00, &mut cartridge);
        memory.write(0x0020, 0x00, &mut cartridge);
        memory.write(0x0028, 0xFF, &mut cartridge);
        memory.write(0x2000, 0x01, &mut cartridge);
        memory.write(0x2400, 0x02, &mut cartridge);
        memory.write(0x3F00, 0x0F, &mut cartridge);
        memory.write(0x3F01, 0x11, &mut cartridge);
        memory.write(0x3F02, 0x22, &mut cartridge);

        let scroll_events = [ScrollEvent {
            scanline: 0,
            dot: 0,
            scroll_x: 0,
            scroll_y: 0,
            base_nametable: 0x0400,
            fine_x_scroll: 0,
            vram_addr: 0x0400,
            temp_vram_addr: 0x0000,
        }];

        assert_eq!(
            background_pixel_at(
                &memory,
                &registers,
                &scroll_events,
                0,
                0,
                0x0000,
                &cartridge
            ),
            (0x11, true)
        );
    }
}
