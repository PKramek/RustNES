pub const CTRL_NMI_ENABLE: u8 = 0x80;
pub const CTRL_VRAM_INCREMENT: u8 = 0x04;
pub const STATUS_SPRITE_OVERFLOW: u8 = 0x20;
pub const STATUS_SPRITE_ZERO_HIT: u8 = 0x40;
pub const STATUS_VBLANK: u8 = 0x80;

#[derive(Debug, Clone, Copy, Default)]
pub struct PpuRegisters {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,
    pub v: u16,
    pub t: u16,
    pub x: u8,
    pub w: bool,
    pub read_buffer: u8,
}

impl PpuRegisters {
    pub fn vram_increment(&self) -> u16 {
        if self.ctrl & CTRL_VRAM_INCREMENT != 0 {
            32
        } else {
            1
        }
    }

    pub fn nmi_enabled(&self) -> bool {
        self.ctrl & CTRL_NMI_ENABLE != 0
    }

    pub fn vblank(&self) -> bool {
        self.status & STATUS_VBLANK != 0
    }

    pub fn set_vblank(&mut self, enabled: bool) {
        if enabled {
            self.status |= STATUS_VBLANK;
        } else {
            self.status &= !STATUS_VBLANK;
        }
    }

    pub fn clear_frame_flags(&mut self) {
        self.status &= !(STATUS_VBLANK | STATUS_SPRITE_ZERO_HIT | STATUS_SPRITE_OVERFLOW);
    }
}