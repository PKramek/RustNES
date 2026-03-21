#[derive(Debug, Clone, Copy, Default)]
pub struct ControllerPort {
    latched_buttons: u8,
    shift_register: u8,
    strobe_high: bool,
}

impl ControllerPort {
    pub fn set_buttons(&mut self, buttons: u8) {
        self.latched_buttons = buttons;
        if self.strobe_high {
            self.shift_register = buttons;
        }
    }

    pub fn write_strobe(&mut self, value: u8) {
        let strobe_high = value & 1 != 0;
        self.strobe_high = strobe_high;
        if strobe_high {
            self.shift_register = self.latched_buttons;
        }
    }

    pub fn read(&mut self) -> u8 {
        let value = self.shift_register & 1;
        if !self.strobe_high {
            self.shift_register >>= 1;
            self.shift_register |= 0x80;
        }
        value | 0x40
    }
}