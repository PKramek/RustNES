#[derive(Debug, Clone, Default)]
pub struct PpuPortsStub {
    ctrl: u8,
    mask: u8,
    status: u8,
    oam_data: u8,
    ppudata: u8,
    write_toggle: bool,
}

impl PpuPortsStub {
    pub fn ctrl(&self) -> u8 {
        self.ctrl
    }

    pub fn set_status(&mut self, value: u8) {
        self.status = value;
    }

    pub fn write_toggle(&self) -> bool {
        self.write_toggle
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x2000 => self.ctrl,
            0x2001 => self.mask,
            0x2002 => {
                let status = self.status;
                self.status &= 0x7F;
                self.write_toggle = false;
                status
            }
            0x2004 => self.oam_data,
            0x2007 => self.ppudata,
            _ => 0,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x2000 => self.ctrl = value,
            0x2001 => self.mask = value,
            0x2004 => self.oam_data = value,
            0x2005 | 0x2006 => self.write_toggle = !self.write_toggle,
            0x2007 => self.ppudata = value,
            _ => {}
        }
    }
}