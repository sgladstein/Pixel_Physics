//! **Mould Organ: the slime mould plays a rhythm.** `cargo run --release --bin organ`
//!
//! A prototype beside `--bin mould`, on the same model (`pixel_physics::mould`,
//! the organ itself in `mould::organ`). One food pile is the heart and three
//! are bells; every two seconds a pulse leaves the heart along the tubes and
//! each bell rings when it arrives, so the length of a tube is the timing of
//! its note. Push a tube with the lamp and its note comes late while you
//! hold it; wall it off with salt and lead it round with oats and it goes
//! silent, then comes back late; join two bells and the loop rings an echo.
//!
//! Keys: 1-4 tools, T target rhythm on/off, P pause, R restart, N next dish.
//! `cargo run --release --bin organ -- 3` starts on dish seed 3.
//!
//! **Sound** plays where the build has a sound device -- macOS and Windows,
//! and Linux with `--features audio-alsa` -- the same rule as the lab's
//! (`pixel_physics::audio`). Elsewhere it runs silent, and the probe's
//! offline WAV (`examples/organprobe.rs`, `play=1 frames=DIR`) is the way
//! to hear it.
//!
//! Headless: `PIXEL_PHYSICS_ORGAN_SCREENSHOT_AFTER_FRAMES=N` writes
//! `pixel_physics_organ_screenshot.png` to the temp directory after N drawn
//! frames, the same hook the other binaries have.

use std::sync::Arc;
use std::time::{Duration, Instant};

use pixel_physics::mould::organ::{self, Note, Organ};
use pixel_physics::mould::render::{self, View};
use pixel_physics::mould::{Tool, H, STEPS_PER_FRAME, W};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

/// Buffer pixels per dish cell. 4 makes a 1024-wide window.
const SCALE: u32 = 4;
const TICK: Duration = Duration::from_nanos(1_000_000_000 / 60);
const MAX_TICKS_PER_FRAME: u32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seed = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut handler = Handler::new(seed);
    event_loop.run_app(&mut handler)?;
    handler.result
}

struct Handler {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    organ: Organ,
    seed: u64,
    view: View,
    cursor: Option<(f32, f32)>,
    pressed: bool,
    last_frame: Instant,
    accumulator: Duration,
    frames: u64,
    screenshot_after: Option<u64>,
    speaker: speaker::Speaker,
    result: Result<(), Box<dyn std::error::Error>>,
}

impl Handler {
    fn new(seed: u64) -> Self {
        let screenshot_after = std::env::var("PIXEL_PHYSICS_ORGAN_SCREENSHOT_AFTER_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok());
        let speaker = speaker::Speaker::open();
        Handler {
            window: None,
            pixels: None,
            organ: Organ::new(seed),
            seed,
            view: View::default(),
            cursor: None,
            pressed: false,
            last_frame: Instant::now(),
            accumulator: Duration::ZERO,
            frames: 0,
            screenshot_after,
            speaker,
            result: Ok(()),
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.result = Err(message.into());
        event_loop.exit();
    }

    fn restart(&mut self, seed: u64) {
        self.seed = seed;
        let target = self.organ.target;
        self.organ = Organ::new(seed);
        self.organ.target = target;
        self.pressed = false;
        if let Some(w) = &self.window {
            w.set_title(&format!("Mould Organ - dish {seed}"));
        }
    }

    fn hold(&mut self) {
        let lamp = match (self.pressed, self.view.tool, self.cursor) {
            (true, Tool::Lamp, Some(c)) => Some(c),
            _ => None,
        };
        self.organ.dish.set_lamp(lamp);
    }

    /// Let go of everything: no button held, no stroke drawing, lamp off.
    /// Used whenever the hand may have left without a release reaching us.
    fn let_go(&mut self) {
        self.pressed = false;
        self.organ.dish.end_stroke();
        self.hold();
    }

    fn mouse(&mut self, down: bool) {
        // A release ends the stroke wherever it lands. It returned first
        // when the cursor was over the HUD strip or outside the window,
        // leaving a salt or snip stroke live for the next lamp press to drag
        // (review 1, finding 5; the mould window's own fix).
        if !down {
            self.let_go();
            return;
        }
        self.pressed = true;
        let Some((x, y)) = self.cursor else { return };
        let d = &mut self.organ.dish;
        match self.view.tool {
            Tool::Oats => {
                d.place_oats(x, y);
            }
            t @ (Tool::Salt | Tool::Snip) => {
                d.begin_stroke(t, x, y);
            }
            _ => {}
        }
        self.hold();
    }

    fn key(&mut self, code: KeyCode, event_loop: &ActiveEventLoop) {
        match code {
            KeyCode::Digit1 => self.view.tool = Tool::Lamp,
            KeyCode::Digit2 => self.view.tool = Tool::Oats,
            KeyCode::Digit3 => self.view.tool = Tool::Salt,
            KeyCode::Digit4 => self.view.tool = Tool::Snip,
            KeyCode::KeyT => self.organ.target = !self.organ.target,
            KeyCode::KeyP => self.view.paused = !self.view.paused,
            KeyCode::KeyR => self.restart(self.seed),
            KeyCode::KeyN => self.restart(self.seed + 1),
            KeyCode::Escape => event_loop.exit(),
            _ => {}
        }
        self.hold();
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.accumulator += now - self.last_frame;
        self.last_frame = now;
        let mut ticks = 0;
        let mut sounded: Vec<Note> = Vec::new();
        while self.accumulator >= TICK && ticks < MAX_TICKS_PER_FRAME {
            self.accumulator -= TICK;
            ticks += 1;
            if !self.view.paused {
                for _ in 0..STEPS_PER_FRAME {
                    sounded.extend(self.organ.step());
                }
            }
        }
        if ticks == MAX_TICKS_PER_FRAME {
            self.accumulator = Duration::ZERO;
        }
        self.speaker.send(&sounded);
        let Some(pixels) = self.pixels.as_mut() else { return };
        self.view.cursor = self.cursor;
        organ::draw(&self.organ, &self.view, SCALE, pixels.frame_mut());
        if let Err(err) = pixels.render() {
            return self.fail(event_loop, format!("render failed: {err}"));
        }
        self.frames += 1;
        if self.screenshot_after == Some(self.frames) {
            let (w, h) = render::frame_size(SCALE);
            let path = std::env::temp_dir().join("pixel_physics_organ_screenshot.png");
            let img = image::RgbaImage::from_raw(w, h, pixels.frame().to_vec());
            match img.map(|i| i.save(&path)) {
                Some(Ok(())) => println!("screenshot written to {}", path.display()),
                other => eprintln!("screenshot failed: {other:?}"),
            }
        }
    }
}

impl ApplicationHandler for Handler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (w, h) = render::frame_size(SCALE);
        let attributes = Window::default_attributes()
            .with_title(format!("Mould Organ - dish {}", self.seed))
            .with_inner_size(LogicalSize::new(w, h))
            .with_min_inner_size(LogicalSize::new(W as u32 * 2, H as u32 * 2));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(err) => return self.fail(event_loop, format!("window creation failed: {err}")),
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        match Pixels::new(w, h, surface) {
            Ok(p) => self.pixels = Some(p),
            Err(err) => return self.fail(event_loop, format!("pixels init failed: {err}")),
        }
        self.window = Some(window);
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    let err = self
                        .pixels
                        .as_mut()
                        .and_then(|p| p.resize_surface(size.width, size.height).err());
                    if let Some(err) = err {
                        self.fail(event_loop, format!("surface resize failed: {err}"));
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if event.state == ElementState::Pressed && !event.repeat {
                        self.key(code, event_loop);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = self
                    .pixels
                    .as_ref()
                    .and_then(|p| p.window_pos_to_pixel((position.x as f32, position.y as f32)).ok())
                    .map(|(x, y)| (x as f32 / SCALE as f32, y as f32 / SCALE as f32))
                    .filter(|&(_, y)| y < H as f32);
                if let Some((x, y)) = self.cursor {
                    if self.pressed && self.organ.dish.stroke_active() {
                        self.organ.dish.drag_stroke(x, y);
                    }
                }
                self.hold();
            }
            // Alt-tab with the button held: the release goes elsewhere.
            WindowEvent::Focused(false) => self.let_go(),
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                self.hold();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.mouse(state == ElementState::Pressed);
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// **The speaker**: `organ::Voices` run in the sound device's own callback,
/// fed notes over a bounded channel the game side fills with `try_send`, so
/// neither side waits on the other -- the shape of `audio::device`, which
/// cannot be reused as it is because it plays the lab's voice bank only.
/// Compiled where `audio::device` is; elsewhere an empty shell that drops
/// the notes.
#[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa")))]
mod speaker {
    use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SizedSample};
    use pixel_physics::mould::organ::{Note, Voices};

    pub struct Speaker {
        live: Option<(cpal::Stream, SyncSender<Note>)>,
    }

    impl Speaker {
        pub fn open() -> Speaker {
            match open() {
                Ok(live) => Speaker { live: Some(live) },
                Err(e) => {
                    eprintln!("organ: no sound ({e}); running silent");
                    Speaker { live: None }
                }
            }
        }

        pub fn send(&mut self, notes: &[Note]) {
            if let Some((_, tx)) = &self.live {
                for n in notes {
                    let _ = tx.try_send(*n);
                }
            }
        }
    }

    fn open() -> Result<(cpal::Stream, SyncSender<Note>), String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| "no output device".to_string())?;
        let supported = device.default_output_config().map_err(|e| format!("no output config: {e}"))?;
        let format = supported.sample_format();
        let config = supported.config();
        let (tx, rx) = sync_channel(256);
        let voices = Voices::new(config.sample_rate);
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, config, voices, rx),
            cpal::SampleFormat::I16 => build::<i16>(&device, config, voices, rx),
            cpal::SampleFormat::U16 => build::<u16>(&device, config, voices, rx),
            cpal::SampleFormat::I32 => build::<i32>(&device, config, voices, rx),
            other => return Err(format!("unsupported sample format {other:?}")),
        }?;
        stream.play().map_err(|e| format!("could not start the stream: {e}"))?;
        Ok((stream, tx))
    }

    fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, mut voices: Voices, rx: Receiver<Note>) -> Result<cpal::Stream, String>
    where
        T: SizedSample + FromSample<f32>,
    {
        let channels = config.channels as usize;
        let mut scratch: Vec<f32> = vec![0.0; 8192 * channels.max(1)];
        device
            .build_output_stream(
                config,
                move |data: &mut [T], _info: &cpal::OutputCallbackInfo| {
                    while let Ok(n) = rx.try_recv() {
                        voices.strike(&n);
                    }
                    if scratch.len() < data.len() {
                        scratch.resize(data.len(), 0.0);
                    }
                    let buf = &mut scratch[..data.len()];
                    buf.fill(0.0);
                    voices.render(buf, channels);
                    for (d, s) in data.iter_mut().zip(buf.iter()) {
                        *d = T::from_sample(*s * 0.6);
                    }
                },
                |e| eprintln!("organ: sound stream error: {e}"),
                None,
            )
            .map_err(|e| format!("could not open the stream: {e}"))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", feature = "audio-alsa"))))]
mod speaker {
    use pixel_physics::mould::organ::Note;

    pub struct Speaker;

    impl Speaker {
        pub fn open() -> Speaker {
            eprintln!("organ: this build has no sound device (see the file's header); running silent");
            Speaker
        }

        pub fn send(&mut self, _notes: &[Note]) {}
    }
}
