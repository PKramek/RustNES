use winit::dpi::PhysicalSize;

use crate::core::ppu::{SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationMode {
    Windowed,
    FullscreenBorderless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    Integer,
    FitWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationState {
    pub mode: PresentationMode,
    pub scale_mode: ScaleMode,
    pub window_scale: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationAction {
    ToggleFullscreen,
    ToggleScaleMode,
}

pub fn default_presentation_state() -> PresentationState {
    PresentationState {
        mode: PresentationMode::Windowed,
        scale_mode: ScaleMode::Integer,
        window_scale: 3,
    }
}

pub fn apply_presentation_action(state: &mut PresentationState, action: PresentationAction) {
    match action {
        PresentationAction::ToggleFullscreen => {
            state.mode = match state.mode {
                PresentationMode::Windowed => PresentationMode::FullscreenBorderless,
                PresentationMode::FullscreenBorderless => PresentationMode::Windowed,
            };
        }
        PresentationAction::ToggleScaleMode => {
            state.scale_mode = match state.scale_mode {
                ScaleMode::Integer => ScaleMode::FitWindow,
                ScaleMode::FitWindow => ScaleMode::Integer,
            };
        }
    }
}

pub fn base_window_size(window_scale: u32) -> PhysicalSize<u32> {
    PhysicalSize::new(
        SCREEN_WIDTH as u32 * window_scale.max(1),
        SCREEN_HEIGHT as u32 * window_scale.max(1),
    )
}
