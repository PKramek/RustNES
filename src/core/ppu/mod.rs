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
use render::render_background_frame;
use sprites::compose_sprites;

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 240;
pub const FRAMEBUFFER_LEN: usize = SCREEN_WIDTH * SCREEN_HEIGHT;
pub const PPU_DOTS_PER_SCANLINE: u16 = 341;
pub const PPU_SCANLINES_PER_FRAME: i16 = 262;
const PRE_RENDER_SCANLINE: i16 = 261;
const VBLANK_START_SCANLINE: i16 = 241;

#[derive(Debug, Clone, Copy)]
pub struct ScrollEvent {
    pub scanline: usize,
    pub scroll_x: u16,
    pub scroll_y: u16,
    pub base_nametable: u16,
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
                scroll_x: 0,
                scroll_y: 0,
                base_nametable: 0,
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
                let status = self.registers.status;
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

        match (self.scanline, self.dot) {
            (VBLANK_START_SCANLINE, 1) => {
                if self.refresh_framebuffer(cartridge) {
                    self.registers.status |= STATUS_SPRITE_ZERO_HIT;
                }
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
            scroll_x: self.scroll_x,
            scroll_y: self.scroll_y,
            base_nametable: ((self.registers.ctrl as u16) & 0x03) << 10,
        }
    }

    fn record_scroll_event(&mut self) {
        let scanline = if (0..SCREEN_HEIGHT as i16).contains(&self.scanline) {
            self.scanline as usize
        } else {
            0
        };
        let event = self.current_scroll_event(scanline);
        match self.scroll_events.last_mut() {
            Some(last) if last.scanline == scanline => *last = event,
            _ => self.scroll_events.push(event),
        }
    }
}
