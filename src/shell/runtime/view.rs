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

    upload_frame(pixels.frame_mut(), session.last_presented_frame());
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
                                let _ = session.handle_runtime_key(
                                    key,
                                    event.state == ElementState::Pressed,
                                    event.repeat,
                                );
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            upload_frame(pixels.frame_mut(), session.last_presented_frame());
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
                                    target.exit();
                                    return;
                                }
                            }
                            Err(error) => {
                                eprintln!(
                                    "RustNES could not advance the current session: {}.",
                                    error
                                );
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

fn upload_frame(target: &mut [u8], source: &[u8; FRAMEBUFFER_LEN]) {
    write_rgba_frame(source, target);
}
