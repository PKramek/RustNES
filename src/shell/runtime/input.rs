use std::collections::HashSet;

use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NesButton {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl NesButton {
    pub const ALL: [Self; 8] = [
        Self::A,
        Self::B,
        Self::Select,
        Self::Start,
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
    ];

    pub fn mask(self) -> u8 {
        match self {
            Self::A => 0x01,
            Self::B => 0x02,
            Self::Select => 0x04,
            Self::Start => 0x08,
            Self::Up => 0x10,
            Self::Down => 0x20,
            Self::Left => 0x40,
            Self::Right => 0x80,
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|button| *button == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|button| *button == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuAction {
    Resume,
    SoftReset,
    ReloadCurrentRom,
    RemapControls,
    AudioControls,
}

impl PauseMenuAction {
    pub const ALL: [Self; 5] = [
        Self::Resume,
        Self::SoftReset,
        Self::ReloadCurrentRom,
        Self::RemapControls,
        Self::AudioControls,
    ];

    pub fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|action| *action == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|action| *action == self)
            .unwrap_or(0);
        Self::ALL[(index + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMenuMode {
    Hidden,
    PauseMenu { selected: PauseMenuAction },
    RemapControls { selected: NesButton },
    AudioControls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBindings {
    pub a: KeyCode,
    pub b: KeyCode,
    pub select: KeyCode,
    pub start: KeyCode,
    pub up: KeyCode,
    pub down: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
}

impl Default for InputBindings {
    fn default() -> Self {
        Self {
            a: KeyCode::KeyX,
            b: KeyCode::KeyZ,
            select: KeyCode::ShiftRight,
            start: KeyCode::Enter,
            up: KeyCode::ArrowUp,
            down: KeyCode::ArrowDown,
            left: KeyCode::ArrowLeft,
            right: KeyCode::ArrowRight,
        }
    }
}

impl InputBindings {
    pub fn key_for(&self, button: NesButton) -> KeyCode {
        match button {
            NesButton::A => self.a,
            NesButton::B => self.b,
            NesButton::Select => self.select,
            NesButton::Start => self.start,
            NesButton::Up => self.up,
            NesButton::Down => self.down,
            NesButton::Left => self.left,
            NesButton::Right => self.right,
        }
    }

    pub fn set_key(&mut self, button: NesButton, key: KeyCode) {
        match button {
            NesButton::A => self.a = key,
            NesButton::B => self.b = key,
            NesButton::Select => self.select = key,
            NesButton::Start => self.start = key,
            NesButton::Up => self.up = key,
            NesButton::Down => self.down = key,
            NesButton::Left => self.left = key,
            NesButton::Right => self.right = key,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AxisDirection {
    Negative,
    Positive,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AxisState {
    negative_pressed: bool,
    positive_pressed: bool,
    last_pressed: Option<AxisDirection>,
}

impl AxisState {
    fn update(&mut self, direction: AxisDirection, pressed: bool) {
        match direction {
            AxisDirection::Negative => self.negative_pressed = pressed,
            AxisDirection::Positive => self.positive_pressed = pressed,
        }

        if pressed {
            self.last_pressed = Some(direction);
        } else if self.last_pressed == Some(direction) {
            self.last_pressed = match (self.negative_pressed, self.positive_pressed) {
                (true, false) => Some(AxisDirection::Negative),
                (false, true) => Some(AxisDirection::Positive),
                _ => None,
            };
        }
    }

    fn resolve(self, negative_mask: u8, positive_mask: u8) -> u8 {
        match (
            self.negative_pressed,
            self.positive_pressed,
            self.last_pressed,
        ) {
            (true, false, _) => negative_mask,
            (false, true, _) => positive_mask,
            (true, true, Some(AxisDirection::Negative)) => negative_mask,
            (true, true, Some(AxisDirection::Positive)) => positive_mask,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    horizontal: AxisState,
    vertical: AxisState,
}

impl InputState {
    pub fn clear(&mut self) {
        self.pressed_keys.clear();
        self.horizontal = AxisState::default();
        self.vertical = AxisState::default();
    }

    pub fn set_key_state(&mut self, key: KeyCode, pressed: bool, bindings: &InputBindings) {
        if pressed {
            self.pressed_keys.insert(key);
        } else {
            self.pressed_keys.remove(&key);
        }

        if key == bindings.left {
            self.horizontal.update(AxisDirection::Negative, pressed);
        }
        if key == bindings.right {
            self.horizontal.update(AxisDirection::Positive, pressed);
        }
        if key == bindings.up {
            self.vertical.update(AxisDirection::Negative, pressed);
        }
        if key == bindings.down {
            self.vertical.update(AxisDirection::Positive, pressed);
        }
    }

    pub fn resolve_button_mask(&self, bindings: &InputBindings) -> u8 {
        let mut mask = 0u8;

        for button in [
            NesButton::A,
            NesButton::B,
            NesButton::Select,
            NesButton::Start,
        ] {
            if self.pressed_keys.contains(&bindings.key_for(button)) {
                mask |= button.mask();
            }
        }

        mask |= self
            .vertical
            .resolve(NesButton::Up.mask(), NesButton::Down.mask());
        mask |= self
            .horizontal
            .resolve(NesButton::Left.mask(), NesButton::Right.mask());
        mask
    }
}
