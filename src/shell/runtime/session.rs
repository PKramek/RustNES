use std::collections::VecDeque;

use thiserror::Error;
use winit::keyboard::KeyCode;

use crate::core::cartridge::Cartridge;
use crate::core::console::Console;
use crate::core::cpu::{CpuError, StepRecord, format_trace_line};
use crate::core::ppu::FRAMEBUFFER_LEN;

use super::super::{LoadRomError, LoadedRom, load_rom_from_path};
use super::audio::{AudioInitError, RuntimeAudio};
use super::input::{InputBindings, InputState, NesButton, PauseMenuAction, RuntimeMenuMode};

const FRAME_STEP_LIMIT: usize = 100_000;
const RECENT_TRACE_CAPACITY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimePreferences {
    pub master_volume: f32,
    pub muted: bool,
}

impl Default for RuntimePreferences {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseState {
    Running,
    Paused,
}

#[derive(Debug, Error)]
pub enum RuntimeActionError {
    #[error("failed to step runtime frame: {source}")]
    FrameAdvance {
        #[source]
        source: CpuError,
    },
    #[error("failed to reload current ROM: {source}")]
    ReloadCurrentRom {
        #[source]
        source: LoadRomError,
    },
}

impl RuntimeActionError {
    pub fn diagnostic_message(&self) -> String {
        match self {
            Self::FrameAdvance { source } => {
                format!("RustNES could not advance the current session: {source}.")
            }
            Self::ReloadCurrentRom { source } => {
                format!(
                    "RustNES could not reload the current ROM: {}.",
                    source.diagnostic_message()
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct RuntimeSession {
    pub rom: LoadedRom,
    console: Console,
    audio: RuntimeAudio,
    pause_state: PauseState,
    preferences: RuntimePreferences,
    bindings: InputBindings,
    input_state: InputState,
    menu_mode: RuntimeMenuMode,
    last_presented_frame: [u8; FRAMEBUFFER_LEN],
    debug_overlay_visible: bool,
    recent_steps: VecDeque<StepRecord>,
}

impl RuntimeSession {
    pub fn new(rom: LoadedRom, cartridge: Cartridge) -> Self {
        let mut console = Console::new(cartridge);
        console.reset();
        let audio = RuntimeAudio::without_output(console.audio_sample_rate());

        let session = Self {
            rom,
            console,
            audio,
            pause_state: PauseState::Running,
            preferences: RuntimePreferences::default(),
            bindings: InputBindings::default(),
            input_state: InputState::default(),
            menu_mode: RuntimeMenuMode::Hidden,
            last_presented_frame: [0; FRAMEBUFFER_LEN],
            debug_overlay_visible: false,
            recent_steps: VecDeque::with_capacity(RECENT_TRACE_CAPACITY),
        };
        let mut session = session;
        session.sync_runtime_state();
        session.refresh_last_presented_frame();
        session
    }

    pub fn console(&self) -> &Console {
        &self.console
    }

    pub fn console_mut(&mut self) -> &mut Console {
        &mut self.console
    }

    pub fn pause_state(&self) -> PauseState {
        self.pause_state
    }

    pub fn is_paused(&self) -> bool {
        self.pause_state == PauseState::Paused
    }

    pub fn set_pause_state(&mut self, pause_state: PauseState) {
        self.pause_state = pause_state;
    }

    pub fn preferences(&self) -> RuntimePreferences {
        self.preferences
    }

    pub fn preferences_mut(&mut self) -> &mut RuntimePreferences {
        &mut self.preferences
    }

    pub fn audio_output_available(&self) -> bool {
        self.audio.output_available()
    }

    pub fn bindings(&self) -> &InputBindings {
        &self.bindings
    }

    pub fn menu_mode(&self) -> RuntimeMenuMode {
        self.menu_mode
    }

    pub fn selected_pause_action(&self) -> PauseMenuAction {
        match self.menu_mode {
            RuntimeMenuMode::PauseMenu { selected } => selected,
            _ => PauseMenuAction::Resume,
        }
    }

    pub fn selected_remap_button(&self) -> Option<NesButton> {
        match self.menu_mode {
            RuntimeMenuMode::RemapControls { selected } => Some(selected),
            _ => None,
        }
    }

    pub fn debug_overlay_visible(&self) -> bool {
        self.debug_overlay_visible
    }

    pub fn toggle_debug_overlay(&mut self) {
        self.debug_overlay_visible = !self.debug_overlay_visible;
    }

    pub fn debug_snapshot_text(&self) -> String {
        let cpu = self.console.cpu();
        let bus = self.console.bus();
        let ppu = bus.ppu();
        let controller1 = bus.controller1();

        let mut lines = vec![
            String::from("RUNTIME DEBUG SNAPSHOT"),
            format!("ROM: {}", self.rom.source_path.display()),
            format!(
                "STATE: pause={:?} menu={:?} volume={:.0}% muted={} debug_hud={}",
                self.pause_state,
                self.menu_mode,
                self.preferences.master_volume * 100.0,
                self.preferences.muted,
                self.debug_overlay_visible,
            ),
            format!(
                "CPU: PC={:04X} A={:02X} X={:02X} Y={:02X} SP={:02X} P={:02X} cycles={} instructions={}",
                cpu.pc,
                cpu.a,
                cpu.x,
                cpu.y,
                cpu.sp,
                cpu.status,
                cpu.total_cycles,
                cpu.instruction_count,
            ),
            format!(
                "PPU: frame={} scanline={} dot={} status={:02X} ctrl={:02X} mask={:02X} frame_ready={} nmi_line={}",
                ppu.frame(),
                ppu.scanline(),
                ppu.dot(),
                ppu.status(),
                ppu.ctrl(),
                ppu.mask(),
                ppu.frame_ready(),
                ppu.nmi_line(),
            ),
            format!(
                "INPUT: resolved_mask={:02X} latched={:02X} shift={:02X} strobe_high={}",
                self.resolved_button_mask(),
                controller1.latched_buttons(),
                controller1.shift_register(),
                controller1.strobe_high(),
            ),
        ];

        if !self.recent_steps.is_empty() {
            lines.push(String::from("RECENT TRACE:"));
            lines.extend(self.recent_trace_lines(12));
        }

        lines.join("\n")
    }

    pub fn recent_trace_lines(&self, limit: usize) -> Vec<String> {
        self.recent_steps
            .iter()
            .rev()
            .take(limit)
            .map(format_trace_line)
            .collect()
    }

    pub fn begin_remap_controls(&mut self) {
        self.pause_state = PauseState::Paused;
        self.menu_mode = RuntimeMenuMode::RemapControls {
            selected: NesButton::A,
        };
        self.input_state.clear();
        self.sync_controller1();
    }

    pub fn remap_button(&mut self, button: NesButton, key: KeyCode) {
        self.bindings.set_key(button, key);
        self.sync_controller1();
    }

    pub fn resolved_button_mask(&self) -> u8 {
        if self.is_paused() {
            0
        } else {
            self.input_state.resolve_button_mask(&self.bindings)
        }
    }

    pub fn last_presented_frame(&self) -> &[u8; FRAMEBUFFER_LEN] {
        &self.last_presented_frame
    }

    pub fn advance_until_next_frame(&mut self) -> Result<bool, CpuError> {
        if self.is_paused() {
            self.sync_audio();
            return Ok(false);
        }

        let start_frame = self.console.bus().ppu().frame();
        let mut advanced = false;
        for _ in 0..FRAME_STEP_LIMIT {
            let record = self.console.step_instruction()?;
            self.record_step(record);
            if self.console.bus().ppu().frame() > start_frame {
                advanced = true;
                break;
            }
        }
        self.sync_audio();
        self.refresh_last_presented_frame();
        Ok(advanced)
    }

    pub fn start_audio_output(&mut self) -> Result<(), AudioInitError> {
        self.start_audio_output_with(RuntimeAudio::new_default)
    }

    pub fn start_audio_output_with<F>(&mut self, create_audio: F) -> Result<(), AudioInitError>
    where
        F: FnOnce(u32) -> Result<RuntimeAudio, AudioInitError>,
    {
        let audio = create_audio(self.console.audio_sample_rate())?;
        self.console.set_audio_sample_rate(audio.sample_rate());
        audio.apply_preferences(self.preferences, self.is_paused());
        self.audio = audio;
        self.sync_audio();
        Ok(())
    }

    pub fn render_offline_audio(&mut self, frame_count: usize, channels: usize) -> Vec<f32> {
        self.audio.render_offline(frame_count, channels)
    }

    pub fn handle_runtime_key(
        &mut self,
        key: KeyCode,
        pressed: bool,
        repeat: bool,
    ) -> Result<(), RuntimeActionError> {
        if pressed && !repeat {
            match key {
                KeyCode::F1 => {
                    self.toggle_debug_overlay();
                    return Ok(());
                }
                KeyCode::F2 => {
                    eprintln!("{}", self.debug_snapshot_text());
                    return Ok(());
                }
                _ => {}
            }
        }

        if key == KeyCode::Escape && pressed && !repeat {
            match self.menu_mode {
                RuntimeMenuMode::Hidden => self.open_pause_menu(),
                RuntimeMenuMode::PauseMenu { .. } => self.resume(),
                RuntimeMenuMode::RemapControls { .. } => {
                    self.handle_remap_controls_key(key);
                }
                RuntimeMenuMode::AudioControls => {
                    self.handle_audio_controls_key(key);
                }
            }
            return Ok(());
        }

        if self.is_paused() {
            if !pressed || repeat {
                return Ok(());
            }

            return match self.menu_mode {
                RuntimeMenuMode::PauseMenu { .. } => self.handle_pause_menu_key(key),
                RuntimeMenuMode::RemapControls { .. } => {
                    self.handle_remap_controls_key(key);
                    Ok(())
                }
                RuntimeMenuMode::AudioControls => {
                    self.handle_audio_controls_key(key);
                    Ok(())
                }
                RuntimeMenuMode::Hidden => Ok(()),
            };
        }

        self.input_state.set_key_state(key, pressed, &self.bindings);
        self.sync_controller1();
        Ok(())
    }

    pub fn open_pause_menu(&mut self) {
        self.pause_state = PauseState::Paused;
        self.menu_mode = RuntimeMenuMode::PauseMenu {
            selected: PauseMenuAction::Resume,
        };
        self.input_state.clear();
        self.sync_runtime_state();
    }

    pub fn resume(&mut self) {
        self.pause_state = PauseState::Running;
        self.menu_mode = RuntimeMenuMode::Hidden;
        self.sync_runtime_state();
    }

    pub fn soft_reset(&mut self) {
        self.console.reset();
        self.recent_steps.clear();
        self.input_state.clear();
        self.refresh_last_presented_frame();
        self.sync_runtime_state();
    }

    pub fn reload_current_rom(&mut self) -> Result<(), RuntimeActionError> {
        let preferences = self.preferences;
        let bindings = self.bindings.clone();
        let pause_state = self.pause_state;
        let menu_mode = self.menu_mode;
        let rom_path = self.rom.source_path.clone();

        let (rom, cartridge) = load_rom_from_path(&rom_path)
            .map_err(|source| RuntimeActionError::ReloadCurrentRom { source })?;

        let mut console = Console::new(cartridge);
        console.set_audio_sample_rate(self.audio.sample_rate());
        console.reset();

        self.rom = rom;
        self.console = console;
        self.preferences = preferences;
        self.bindings = bindings;
        self.pause_state = pause_state;
        self.menu_mode = menu_mode;
        self.recent_steps.clear();
        self.input_state.clear();
        self.refresh_last_presented_frame();
        self.sync_runtime_state();
        Ok(())
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        self.preferences.master_volume = (self.preferences.master_volume + delta).clamp(0.0, 1.0);
        self.sync_audio_state();
    }

    pub fn toggle_mute(&mut self) {
        self.preferences.muted = !self.preferences.muted;
        self.sync_audio_state();
    }

    pub fn refresh_last_presented_frame(&mut self) {
        self.last_presented_frame = *self.console.bus().ppu().framebuffer();
    }

    fn sync_controller1(&mut self) {
        let mask = self.resolved_button_mask();
        self.console.bus_mut().controller1_mut().set_buttons(mask);
    }

    fn sync_audio(&mut self) {
        self.sync_audio_state();
        let samples = self.console.take_audio_samples();
        self.audio.push_samples(&samples);
    }

    fn sync_audio_state(&self) {
        self.audio
            .apply_preferences(self.preferences, self.is_paused());
    }

    fn sync_runtime_state(&mut self) {
        self.sync_controller1();
        self.sync_audio();
    }

    fn record_step(&mut self, record: StepRecord) {
        if self.recent_steps.len() == RECENT_TRACE_CAPACITY {
            let _ = self.recent_steps.pop_front();
        }
        self.recent_steps.push_back(record);
    }

    fn handle_pause_menu_key(&mut self, key: KeyCode) -> Result<(), RuntimeActionError> {
        match key {
            KeyCode::ArrowUp => {
                self.menu_mode = RuntimeMenuMode::PauseMenu {
                    selected: self.selected_pause_action().previous(),
                };
            }
            KeyCode::ArrowDown => {
                self.menu_mode = RuntimeMenuMode::PauseMenu {
                    selected: self.selected_pause_action().next(),
                };
            }
            KeyCode::Enter => match self.selected_pause_action() {
                PauseMenuAction::Resume => self.resume(),
                PauseMenuAction::SoftReset => self.soft_reset(),
                PauseMenuAction::ReloadCurrentRom => {
                    self.reload_current_rom()?;
                }
                PauseMenuAction::RemapControls => self.begin_remap_controls(),
                PauseMenuAction::AudioControls => {
                    self.menu_mode = RuntimeMenuMode::AudioControls;
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn handle_remap_controls_key(&mut self, key: KeyCode) {
        let RuntimeMenuMode::RemapControls { selected } = self.menu_mode else {
            return;
        };

        match key {
            KeyCode::ArrowUp => {
                self.menu_mode = RuntimeMenuMode::RemapControls {
                    selected: selected.previous(),
                };
            }
            KeyCode::ArrowDown => {
                self.menu_mode = RuntimeMenuMode::RemapControls {
                    selected: selected.next(),
                };
            }
            KeyCode::Escape => {
                self.menu_mode = RuntimeMenuMode::PauseMenu {
                    selected: PauseMenuAction::RemapControls,
                };
            }
            other => self.remap_button(selected, other),
        }
    }

    fn handle_audio_controls_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::ArrowLeft => self.adjust_volume(-0.1),
            KeyCode::ArrowRight => self.adjust_volume(0.1),
            KeyCode::KeyM => self.toggle_mute(),
            KeyCode::Escape => {
                self.menu_mode = RuntimeMenuMode::PauseMenu {
                    selected: PauseMenuAction::AudioControls,
                };
            }
            _ => {}
        }
    }
}
