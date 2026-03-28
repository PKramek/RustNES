mod audio;
mod input;
mod presentation;
mod session;
mod view;

pub use audio::{AudioInitError, RuntimeAudio};
pub use input::{InputBindings, InputState, NesButton, PauseMenuAction, RuntimeMenuMode};
pub use presentation::{
    PresentationAction, PresentationMode, PresentationState, ScaleMode, apply_presentation_action,
    default_presentation_state,
};
pub use session::{PauseState, RuntimeActionError, RuntimePreferences, RuntimeSession};
pub use view::{
    RuntimeBootstrapError, compose_runtime_frame, presentation_action_for_key, run,
    window_size_for_presentation,
};
