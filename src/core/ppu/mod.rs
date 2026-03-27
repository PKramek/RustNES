mod memory;
mod palette;
mod registers;
mod render;
mod sprites;

use crate::core::cartridge::Cartridge;

use memory::PpuMemory;
pub use palette::{RGBA_PIXEL_BYTES, palette_rgba, write_rgba_frame};
use registers::PpuRegisters;
pub use registers::{
    CTRL_NMI_ENABLE, CTRL_VRAM_INCREMENT, STATUS_SPRITE_OVERFLOW, STATUS_SPRITE_ZERO_HIT,
    STATUS_VBLANK,
};
use render::{background_pixel_at, render_background_frame};
use sprites::{compose_sprites, sprite_zero_opaque_at};

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;
pub const FRAMEBUFFER_LEN: usize = SCREEN_WIDTH * SCREEN_HEIGHT;
pub const PPU_DOTS_PER_SCANLINE: u16 = 341;
pub const PPU_SCANLINES_PER_FRAME: i16 = 262;
const PRE_RENDER_SCANLINE: i16 = 261;
const VBLANK_START_SCANLINE: i16 = 241;
const CPU_VISIBLE_PPU_PHASE_OFFSET: u16 = 3;

#[derive(Debug, Clone, Copy)]
pub struct ScrollEvent {
    pub scanline: usize,
    pub dot: u16,
    pub scroll_x: u16,
    pub scroll_y: u16,
    pub base_nametable: u16,
    pub fine_x_scroll: u8,
    pub temp_vram_addr: u16,
}

#[derive(Debug)]
pub struct Ppu {
    registers: PpuRegisters,
    memory: PpuMemory,
    framebuffer: [u8; FRAMEBUFFER_LEN],
    background_opaque: [bool; FRAMEBUFFER_LEN],
    frame_ready: bool,
    scroll_x: u16,
    scroll_y: u16,
    scroll_events: Vec<ScrollEvent>,
    scanline: i16,
    dot: u16,
    frame: u64,
    total_cycles: u64,
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            registers: PpuRegisters::default(),
            memory: PpuMemory::default(),
            framebuffer: [0; FRAMEBUFFER_LEN],
            background_opaque: [false; FRAMEBUFFER_LEN],
            frame_ready: false,
            scroll_x: 0,
            scroll_y: 0,
            scroll_events: vec![ScrollEvent {
                scanline: 0,
                dot: 0,
                scroll_x: 0,
                scroll_y: 0,
                base_nametable: 0,
                fine_x_scroll: 0,
                temp_vram_addr: 0,
            }],
            scanline: 0,
            dot: 0,
            frame: 0,
            total_cycles: 0,
        }
    }
}

impl Ppu {
    pub fn ctrl(&self) -> u8 {
        self.registers.ctrl
    }

    pub fn mask(&self) -> u8 {
        self.registers.mask
    }

    pub fn status(&self) -> u8 {
        self.registers.status
    }

    pub fn set_status(&mut self, value: u8) {
        self.registers.status = value;
    }

    pub fn oam_addr(&self) -> u8 {
        self.registers.oam_addr
    }

    pub fn write_toggle(&self) -> bool {
        self.registers.w
    }

    pub fn vram_addr(&self) -> u16 {
        self.registers.v & 0x3FFF
    }

    pub fn temp_vram_addr(&self) -> u16 {
        self.registers.t & 0x3FFF
    }

    pub fn fine_x_scroll(&self) -> u8 {
        self.registers.x & 0x07
    }

    pub fn scroll_x(&self) -> u16 {
        self.scroll_x
    }

    pub fn scroll_y(&self) -> u16 {
        self.scroll_y
    }

    pub fn scanline(&self) -> i16 {
        self.scanline
    }

    pub fn dot(&self) -> u16 {
        self.dot
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    pub fn framebuffer(&self) -> &[u8; FRAMEBUFFER_LEN] {
        &self.framebuffer
    }

    pub fn refresh_framebuffer(&mut self, cartridge: &Cartridge) -> bool {
        render_background_frame(
            &self.memory,
            &self.registers,
            &self.scroll_events,
            &mut self.framebuffer,
            &mut self.background_opaque,
            cartridge,
        );
        compose_sprites(
            &self.memory,
            &self.registers,
            &mut self.framebuffer,
            &self.background_opaque,
            cartridge,
        )
    }

    pub fn frame_ready(&self) -> bool {
        self.frame_ready
    }

    pub fn take_frame_ready(&mut self) -> bool {
        let ready = self.frame_ready;
        self.frame_ready = false;
        ready
    }

    pub fn peek_memory(&self, addr: u16, cartridge: &Cartridge) -> u8 {
        self.memory.peek(addr, cartridge)
    }

    pub fn peek_oam(&self, addr: u8) -> u8 {
        self.memory.oam_read(addr)
    }

    pub fn scroll_events(&self) -> &[ScrollEvent] {
        &self.scroll_events
    }

    pub fn oam_dma(&mut self, page_data: &[u8; 256]) {
        for (offset, value) in page_data.iter().enumerate() {
            let addr = self.registers.oam_addr.wrapping_add(offset as u8);
            self.memory.oam_write(addr, *value);
        }
    }

    pub fn cpu_read_register(&mut self, addr: u16, cartridge: &Cartridge) -> u8 {
        match addr {
            0x2002 => {
                let status = self.cpu_visible_status();
                self.registers.set_vblank(false);
                self.registers.w = false;
                status
            }
            0x2004 => self.memory.oam_read(self.registers.oam_addr),
            0x2007 => self.read_ppudata(cartridge),
            _ => 0,
        }
    }

    pub fn cpu_write_register(&mut self, addr: u16, value: u8, cartridge: &mut Cartridge) {
        match addr {
            0x2000 => {
                self.registers.ctrl = value;
                self.registers.t = (self.registers.t & !0x0C00) | (((value as u16) & 0x03) << 10);
                self.record_scroll_event();
            }
            0x2001 => self.registers.mask = value,
            0x2003 => self.registers.oam_addr = value,
            0x2004 => {
                self.memory.oam_write(self.registers.oam_addr, value);
                self.registers.oam_addr = self.registers.oam_addr.wrapping_add(1);
            }
            0x2005 => self.write_ppuscroll(value),
            0x2006 => self.write_ppuaddr(value),
            0x2007 => self.write_ppudata(value, cartridge),
            _ => {}
        }
    }

    pub fn tick(&mut self, cartridge: &Cartridge) {
        self.total_cycles += 1;
        self.dot += 1;

        if self.dot >= PPU_DOTS_PER_SCANLINE {
            self.dot = 0;
            self.scanline += 1;
            if self.scanline >= PPU_SCANLINES_PER_FRAME {
                self.scanline = 0;
                self.frame += 1;
            }
        }

        self.update_sprite_zero_hit(cartridge);

        match (self.scanline, self.dot) {
            (VBLANK_START_SCANLINE, 1) => {
                let _ = self.refresh_framebuffer(cartridge);
                self.registers.set_vblank(true);
                self.frame_ready = true;
            }
            (PRE_RENDER_SCANLINE, 1) => {
                self.registers.clear_frame_flags();
                self.scroll_events.clear();
                self.scroll_events.push(self.current_scroll_event(0));
            }
            _ => {}
        }
    }

    pub fn nmi_line(&self) -> bool {
        self.registers.nmi_enabled() && self.registers.vblank()
    }

    fn read_ppudata(&mut self, cartridge: &Cartridge) -> u8 {
        let addr = self.vram_addr();
        let value = self.memory.read(addr, cartridge);
        let result = if addr >= 0x3F00 {
            self.registers.read_buffer = self.memory.read(addr.wrapping_sub(0x1000), cartridge);
            value
        } else {
            let buffered = self.registers.read_buffer;
            self.registers.read_buffer = value;
            buffered
        };
        self.increment_vram_addr();
        result
    }

    fn write_ppudata(&mut self, value: u8, cartridge: &mut Cartridge) {
        let addr = self.vram_addr();
        self.memory.write(addr, value, cartridge);
        self.increment_vram_addr();
    }

    fn write_ppuscroll(&mut self, value: u8) {
        if !self.registers.w {
            self.registers.t = (self.registers.t & !0x001F) | (((value as u16) >> 3) & 0x001F);
            self.registers.x = value & 0x07;
            self.scroll_x = value as u16;
            self.registers.w = true;
        } else {
            self.registers.t = (self.registers.t & !0x73E0)
                | ((((value as u16) >> 3) & 0x001F) << 5)
                | (((value as u16) & 0x07) << 12);
            self.scroll_y = value as u16;
            self.registers.w = false;
            self.record_scroll_event();
        }
    }

    fn write_ppuaddr(&mut self, value: u8) {
        if !self.registers.w {
            self.registers.t = (self.registers.t & 0x00FF) | (((value as u16) & 0x3F) << 8);
            self.registers.w = true;
        } else {
            self.registers.t = (self.registers.t & 0x7F00) | value as u16;
            self.registers.v = self.registers.t;
            self.registers.w = false;
            self.record_scroll_event();
        }
    }

    fn increment_vram_addr(&mut self) {
        self.registers.v = self
            .registers
            .v
            .wrapping_add(self.registers.vram_increment())
            & 0x3FFF;
    }

    fn current_scroll_event(&self, scanline: usize) -> ScrollEvent {
        ScrollEvent {
            scanline,
            dot: self.dot,
            scroll_x: self.scroll_x,
            scroll_y: self.scroll_y,
            base_nametable: ((self.registers.ctrl as u16) & 0x03) << 10,
            fine_x_scroll: self.registers.x & 0x07,
            temp_vram_addr: self.registers.t & 0x3FFF,
        }
    }

    fn record_scroll_event(&mut self) {
        let visible_scanline = (0..SCREEN_HEIGHT as i16).contains(&self.scanline);
        let scanline = if visible_scanline {
            self.scanline as usize
        } else {
            0
        };
        let mut event = self.current_scroll_event(scanline);
        if !visible_scanline {
            event.dot = 0;
        }
        match self.scroll_events.last_mut() {
            Some(last) if last.scanline == scanline && last.dot == event.dot => *last = event,
            _ => self.scroll_events.push(event),
        }
    }

    fn cpu_visible_status(&self) -> u8 {
        let mut status = self.registers.status;
        let (future_scanline, future_dot) =
            self.advance_position(self.scanline, self.dot, CPU_VISIBLE_PPU_PHASE_OFFSET);

        if self.position_passes_vblank_edge(self.scanline, self.dot, future_scanline, future_dot) {
            status |= STATUS_VBLANK;
        }

        if self.position_passes_pre_render_clear_edge(
            self.scanline,
            self.dot,
            future_scanline,
            future_dot,
        ) {
            status &= !STATUS_VBLANK;
        }

        status
    }

    fn advance_position(&self, mut scanline: i16, mut dot: u16, steps: u16) -> (i16, u16) {
        for _ in 0..steps {
            dot += 1;
            if dot >= PPU_DOTS_PER_SCANLINE {
                dot = 0;
                scanline += 1;
                if scanline >= PPU_SCANLINES_PER_FRAME {
                    scanline = 0;
                }
            }
        }
        (scanline, dot)
    }

    fn position_passes_vblank_edge(
        &self,
        start_scanline: i16,
        start_dot: u16,
        end_scanline: i16,
        end_dot: u16,
    ) -> bool {
        self.position_in_open_closed_range(
            (VBLANK_START_SCANLINE, 1),
            (start_scanline, start_dot),
            (end_scanline, end_dot),
        )
    }

    fn position_passes_pre_render_clear_edge(
        &self,
        start_scanline: i16,
        start_dot: u16,
        end_scanline: i16,
        end_dot: u16,
    ) -> bool {
        self.position_in_open_closed_range(
            (PRE_RENDER_SCANLINE, 1),
            (start_scanline, start_dot),
            (end_scanline, end_dot),
        )
    }

    fn position_in_open_closed_range(
        &self,
        target: (i16, u16),
        start: (i16, u16),
        end: (i16, u16),
    ) -> bool {
        let target_index = self.position_index(target.0, target.1);
        let start_index = self.position_index(start.0, start.1);
        let mut end_index = self.position_index(end.0, end.1);

        if end_index <= start_index {
            end_index += self.frame_position_count();
        }

        let mut target_index = target_index;
        if target_index <= start_index {
            target_index += self.frame_position_count();
        }

        start_index < target_index && target_index <= end_index
    }

    fn position_index(&self, scanline: i16, dot: u16) -> u32 {
        scanline as u32 * PPU_DOTS_PER_SCANLINE as u32 + dot as u32
    }

    fn frame_position_count(&self) -> u32 {
        PPU_SCANLINES_PER_FRAME as u32 * PPU_DOTS_PER_SCANLINE as u32
    }

    fn update_sprite_zero_hit(&mut self, cartridge: &Cartridge) {
        if self.registers.status & STATUS_SPRITE_ZERO_HIT != 0 {
            return;
        }

        if self.scanline < 0 || self.scanline >= SCREEN_HEIGHT as i16 {
            return;
        }

        if self.dot == 0 || self.dot > SCREEN_WIDTH as u16 {
            return;
        }

        let screen_x = (self.dot - 1) as usize;
        let screen_y = self.scanline as usize;

        if screen_x == SCREEN_WIDTH - 1 {
            return;
        }

        let pattern_base = if self.registers.ctrl & render::CTRL_BACKGROUND_PATTERN_TABLE != 0 {
            0x1000
        } else {
            0x0000
        };
        let (_, background_opaque) = background_pixel_at(
            &self.memory,
            &self.registers,
            &self.scroll_events,
            screen_x,
            screen_y,
            pattern_base,
            cartridge,
        );

        if background_opaque
            && sprite_zero_opaque_at(&self.memory, &self.registers, screen_x, screen_y, cartridge)
        {
            self.registers.status |= STATUS_SPRITE_ZERO_HIT;
        }
    }
}
