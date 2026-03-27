use std::sync::Arc;
use std::time::{Duration, Instant};

use pixels::{Error as PixelsError, Pixels, SurfaceTexture};
use thiserror::Error;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowBuilder};

use crate::core::ppu::{FRAMEBUFFER_LEN, SCREEN_HEIGHT, SCREEN_WIDTH, write_rgba_frame};

use super::session::RuntimeSession;
use super::{NesButton, PauseMenuAction, PauseState, RuntimeMenuMode};

const WINDOW_SCALE: f64 = 3.0;
const FRAME_DURATION: Duration = Duration::from_nanos(1_000_000_000 / 60);
const MAX_FRAME_CATCH_UP: usize = 3;

#[derive(Debug, Error)]
pub enum RuntimeBootstrapError {
    #[error("failed to create runtime event loop: {source}")]
    EventLoop {
        #[source]
        source: EventLoopError,
    },
    #[error("failed to create runtime window: {source}")]
    Window {
        #[source]
        source: OsError,
    },
    #[error("failed to create runtime pixel surface: {source}")]
    Pixels {
        #[source]
        source: PixelsError,
    },
}

impl RuntimeBootstrapError {
    pub fn diagnostic_message(&self) -> String {
        format!("RustNES could not start the runtime view: {self}.")
    }
}

pub fn run(mut session: RuntimeSession) -> Result<(), RuntimeBootstrapError> {
    let event_loop =
        EventLoop::new().map_err(|source| RuntimeBootstrapError::EventLoop { source })?;
    let window = Arc::new(build_window(&event_loop, &session)?);
    let mut pixels = build_pixels(window.clone())?;

    if let Err(error) = session.start_audio_output() {
        eprintln!("{}", error.diagnostic_message());
    }

    let initial_frame = compose_runtime_frame(&session);
    upload_frame(pixels.frame_mut(), &initial_frame);
    let mut next_frame_deadline = Instant::now();

    event_loop
        .run(move |event, target| {
            target.set_control_flow(ControlFlow::WaitUntil(next_frame_deadline));

            match event {
                Event::WindowEvent { window_id, event } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => target.exit(),
                        WindowEvent::KeyboardInput { event, .. } => {
                            if let PhysicalKey::Code(key) = event.physical_key {
                                let pause_before = session.pause_state();
                                let menu_before = session.menu_mode();
                                let preferences_before = session.preferences();
                                let debug_before = session.debug_overlay_visible();

                                let _ = session.handle_runtime_key(
                                    key,
                                    event.state == ElementState::Pressed,
                                    event.repeat,
                                );

                                if pause_before != session.pause_state()
                                    || menu_before != session.menu_mode()
                                    || preferences_before != session.preferences()
                                    || debug_before != session.debug_overlay_visible()
                                {
                                    window.request_redraw();
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            let frame = compose_runtime_frame(&session);
                            upload_frame(pixels.frame_mut(), &frame);
                            if pixels.render().is_err() {
                                target.exit();
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    let mut advanced_any_frame = false;
                    let now = Instant::now();
                    let mut catch_up_frames = 0usize;

                    while now >= next_frame_deadline && catch_up_frames < MAX_FRAME_CATCH_UP {
                        match session.advance_until_next_frame() {
                            Ok(true) => {
                                advanced_any_frame = true;
                            }
                            Ok(false) => {
                                if !session.is_paused() {
                                    eprintln!(
                                        "RustNES runtime stopped advancing frames; closing instead of freezing the last frame."
                                    );
                                    eprintln!("{}", session.debug_snapshot_text());
                                    target.exit();
                                    return;
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "RustNES could not advance the current session: {}.",
                                    error
                                );
                                eprintln!("{}", session.debug_snapshot_text());
                                target.exit();
                                return;
                            }
                        }

                        next_frame_deadline += FRAME_DURATION;
                        catch_up_frames += 1;
                    }

                    if catch_up_frames == MAX_FRAME_CATCH_UP && Instant::now() >= next_frame_deadline {
                        next_frame_deadline = Instant::now() + FRAME_DURATION;
                    }

                    if advanced_any_frame {
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        })
        .map_err(|source| RuntimeBootstrapError::EventLoop { source })
}

fn build_window(
    event_loop: &EventLoop<()>,
    session: &RuntimeSession,
) -> Result<Window, RuntimeBootstrapError> {
    WindowBuilder::new()
        .with_title(runtime_title(session))
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(
            SCREEN_WIDTH as f64 * WINDOW_SCALE,
            SCREEN_HEIGHT as f64 * WINDOW_SCALE,
        ))
        .build(event_loop)
        .map_err(|source| RuntimeBootstrapError::Window { source })
}

fn build_pixels(window: Arc<Window>) -> Result<Pixels<'static>, RuntimeBootstrapError> {
    let size = window.inner_size();
    let surface_texture = SurfaceTexture::new(size.width, size.height, window);
    Pixels::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, surface_texture)
        .map_err(|source| RuntimeBootstrapError::Pixels { source })
}

fn runtime_title(session: &RuntimeSession) -> String {
    match &session.rom.title {
        Some(title) => format!("RustNES - {title}"),
        None => format!("RustNES - {}", session.rom.source_path.display()),
    }
}

pub fn compose_runtime_frame(session: &RuntimeSession) -> [u8; FRAMEBUFFER_LEN] {
    let mut frame = *session.last_presented_frame();
    if session.pause_state() == PauseState::Paused {
        let overlay = OverlayLayout {
            x: 24,
            y: 20,
            width: 208,
            height: 188,
        };

        draw_panel(&mut frame, overlay);

        match session.menu_mode() {
            RuntimeMenuMode::Hidden => {}
            RuntimeMenuMode::PauseMenu { selected } => {
                draw_pause_menu(&mut frame, overlay, selected)
            }
            RuntimeMenuMode::RemapControls { .. } => {
                draw_remap_controls(&mut frame, overlay, session)
            }
            RuntimeMenuMode::AudioControls => draw_audio_controls(&mut frame, overlay, session),
        }
    }

    if session.debug_overlay_visible() {
        draw_debug_overlay(&mut frame, session);
    }

    frame
}

fn upload_frame(target: &mut [u8], source: &[u8; FRAMEBUFFER_LEN]) {
    write_rgba_frame(source, target);
}

#[derive(Clone, Copy)]
struct OverlayLayout {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

const PANEL_BG: u8 = 0x0F;
const PANEL_BORDER: u8 = 0x30;
const PANEL_ACCENT: u8 = 0x21;
const TEXT_LIGHT: u8 = 0x20;
const TEXT_HIGHLIGHT: u8 = 0x38;

fn draw_debug_overlay(frame: &mut [u8; FRAMEBUFFER_LEN], session: &RuntimeSession) {
    let overlay = OverlayLayout {
        x: 4,
        y: 4,
        width: 248,
        height: 92,
    };
    draw_panel(frame, overlay);

    let lines = debug_overlay_lines(session);
    for (index, line) in lines.iter().enumerate() {
        draw_text(
            frame,
            overlay.x + 6,
            overlay.y + 6 + index * 10,
            line,
            if index == 0 {
                TEXT_HIGHLIGHT
            } else {
                TEXT_LIGHT
            },
            1,
        );
    }
}

fn debug_overlay_lines(session: &RuntimeSession) -> Vec<String> {
    let cpu = session.console().cpu();
    let bus = session.console().bus();
    let ppu = bus.ppu();
    let pad = bus.controller1().latched_buttons();

    let mut lines = vec![
        String::from("F1 HUD F2 DUMP"),
        format!(
            "PC:{:04X} A:{:02X} X:{:02X} Y:{:02X}",
            cpu.pc, cpu.a, cpu.x, cpu.y
        ),
        format!(
            "SP:{:02X} P:{:02X} CYC:{}",
            cpu.sp, cpu.status, cpu.total_cycles
        ),
        format!("FR:{} SC:{} DT:{}", ppu.frame(), ppu.scanline(), ppu.dot()),
        format!(
            "STAT:{:02X} CTRL:{:02X} MASK:{:02X}",
            ppu.status(),
            ppu.ctrl(),
            ppu.mask()
        ),
        format!(
            "PAD:{:02X} {} {}",
            pad,
            pause_label(session.pause_state()),
            menu_label(session.menu_mode()),
        ),
    ];

    for record in session.recent_trace_lines(3) {
        lines.push(compact_trace_line(&record));
    }

    lines
}

fn pause_label(state: PauseState) -> &'static str {
    match state {
        PauseState::Running => "RUN",
        PauseState::Paused => "PAUSE",
    }
}

fn menu_label(mode: RuntimeMenuMode) -> &'static str {
    match mode {
        RuntimeMenuMode::Hidden => "LIVE",
        RuntimeMenuMode::PauseMenu { .. } => "MENU",
        RuntimeMenuMode::RemapControls { .. } => "REMAP",
        RuntimeMenuMode::AudioControls => "AUDIO",
    }
}

fn compact_trace_line(line: &str) -> String {
    line.chars()
        .map(|ch| match ch {
            'a'..='z' => ch.to_ascii_uppercase(),
            'A'..='Z' | '0'..='9' | ' ' | ':' => ch,
            _ => ' ',
        })
        .take(34)
        .collect()
}

fn draw_pause_menu(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    overlay: OverlayLayout,
    selected: PauseMenuAction,
) {
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 14,
        "PAUSED",
        TEXT_HIGHLIGHT,
        2,
    );

    let actions = [
        PauseMenuAction::Resume,
        PauseMenuAction::SoftReset,
        PauseMenuAction::ReloadCurrentRom,
        PauseMenuAction::RemapControls,
        PauseMenuAction::AudioControls,
    ];

    for (index, action) in actions.iter().enumerate() {
        let row_y = overlay.y + 44 + index * 24;
        let active = *action == selected;
        if active {
            fill_rect(
                frame,
                overlay.x + 12,
                row_y - 4,
                overlay.width - 24,
                18,
                PANEL_ACCENT,
            );
        }
        draw_text(
            frame,
            overlay.x + 22,
            row_y,
            pause_menu_label(*action),
            if active { TEXT_HIGHLIGHT } else { TEXT_LIGHT },
            1,
        );
    }
}

fn draw_remap_controls(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    overlay: OverlayLayout,
    session: &RuntimeSession,
) {
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 14,
        "REMAP CONTROLS",
        TEXT_HIGHLIGHT,
        1,
    );
    if let Some(selected) = session.selected_remap_button() {
        let selected_label = format!("SELECTED: {}", remap_button_label(selected));
        draw_text(
            frame,
            overlay.x + 16,
            overlay.y + 40,
            &selected_label,
            TEXT_LIGHT,
            1,
        );
    }

    for (index, button) in NesButton::ALL.iter().enumerate() {
        let row_y = overlay.y + 66 + index * 14;
        let active = session.selected_remap_button() == Some(*button);
        if active {
            fill_rect(
                frame,
                overlay.x + 12,
                row_y - 3,
                overlay.width - 24,
                12,
                PANEL_ACCENT,
            );
        }
        draw_text(
            frame,
            overlay.x + 18,
            row_y,
            remap_button_label(*button),
            if active { TEXT_HIGHLIGHT } else { TEXT_LIGHT },
            1,
        );
    }
}

fn draw_audio_controls(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    overlay: OverlayLayout,
    session: &RuntimeSession,
) {
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 14,
        "AUDIO CONTROLS",
        TEXT_HIGHLIGHT,
        1,
    );
    let preferences = session.preferences();
    let mute_line = if preferences.muted {
        "MUTE: ON"
    } else {
        "MUTE: OFF"
    };
    let volume_line = format!(
        "VOLUME: {}%",
        (preferences.master_volume * 100.0).round() as u8
    );
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 44,
        mute_line,
        TEXT_LIGHT,
        1,
    );
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 68,
        &volume_line,
        TEXT_LIGHT,
        1,
    );
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 104,
        "LEFT/RIGHT ADJUST",
        TEXT_LIGHT,
        1,
    );
    draw_text(
        frame,
        overlay.x + 16,
        overlay.y + 128,
        "M TOGGLE MUTE",
        TEXT_LIGHT,
        1,
    );
}

fn draw_panel(frame: &mut [u8; FRAMEBUFFER_LEN], overlay: OverlayLayout) {
    fill_rect(
        frame,
        overlay.x,
        overlay.y,
        overlay.width,
        overlay.height,
        PANEL_BG,
    );
    stroke_rect(
        frame,
        overlay.x,
        overlay.y,
        overlay.width,
        overlay.height,
        PANEL_BORDER,
    );
}

fn fill_rect(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u8,
) {
    let max_x = (x + width).min(SCREEN_WIDTH);
    let max_y = (y + height).min(SCREEN_HEIGHT);
    for row in y..max_y {
        let start = row * SCREEN_WIDTH + x;
        let end = row * SCREEN_WIDTH + max_x;
        frame[start..end].fill(color);
    }
}

fn stroke_rect(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u8,
) {
    fill_rect(frame, x, y, width, 2, color);
    fill_rect(frame, x, y + height.saturating_sub(2), width, 2, color);
    fill_rect(frame, x, y, 2, height, color);
    fill_rect(frame, x + width.saturating_sub(2), y, 2, height, color);
}

fn draw_text(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    x: usize,
    y: usize,
    text: &str,
    color: u8,
    scale: usize,
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_glyph(frame, cursor_x, y, ch, color, scale);
        cursor_x += (glyph_width(ch) + 1) * scale;
    }
}

fn draw_glyph(
    frame: &mut [u8; FRAMEBUFFER_LEN],
    x: usize,
    y: usize,
    ch: char,
    color: u8,
    scale: usize,
) {
    let glyph = glyph_rows(ch);
    for (row, bits) in glyph.iter().enumerate() {
        for column in 0..5 {
            if bits & (1 << (4 - column)) == 0 {
                continue;
            }
            fill_rect(
                frame,
                x + column * scale,
                y + row * scale,
                scale,
                scale,
                color,
            );
        }
    }
}

fn glyph_width(ch: char) -> usize {
    if ch == ' ' { 3 } else { 5 }
}

fn pause_menu_label(action: PauseMenuAction) -> &'static str {
    match action {
        PauseMenuAction::Resume => "RESUME",
        PauseMenuAction::SoftReset => "SOFT RESET",
        PauseMenuAction::ReloadCurrentRom => "RELOAD CURRENT ROM",
        PauseMenuAction::RemapControls => "REMAP CONTROLS",
        PauseMenuAction::AudioControls => "AUDIO CONTROLS",
    }
}

fn remap_button_label(button: NesButton) -> &'static str {
    match button {
        NesButton::A => "A",
        NesButton::B => "B",
        NesButton::Select => "SELECT",
        NesButton::Start => "START",
        NesButton::Up => "UP",
        NesButton::Down => "DOWN",
        NesButton::Left => "LEFT",
        NesButton::Right => "RIGHT",
    }
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0F, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x1F, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        '%' => [0x18, 0x19, 0x02, 0x04, 0x08, 0x13, 0x03],
        '/' => [0x01, 0x02, 0x04, 0x04, 0x08, 0x10, 0x00],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x00, 0x08],
    }
}
