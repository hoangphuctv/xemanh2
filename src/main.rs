#![cfg_attr(windows, windows_subsystem = "windows")]

use softbuffer::{Context, Surface};
use std::{
    env,
    fs::OpenOptions,
    io::{BufReader, Write},
    num::NonZeroU32,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use image::AnimationDecoder;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

static DEBUG_LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn init_debug_log(enabled: bool) {
    if !enabled {
        return;
    }

    let path = match env::current_dir() {
        Ok(dir) => dir.join("xemanh2.log"),
        Err(_) => return,
    };

    if let Ok(file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        let mut file = file;
        let _ = writeln!(file, "[DEBUG] log file: {}", path.display());
        let _ = file.flush();
        *DEBUG_LOG.lock().unwrap() = Some(file);
    }
}

fn debug_log(message: String) {
    if let Some(file) = DEBUG_LOG.lock().unwrap().as_mut() {
        let _ = writeln!(file, "{message}");
        let _ = file.flush();
    }
}

macro_rules! debug_log {
    ($($arg:tt)*) => {
        debug_log(format!($($arg)*));
    };
}

struct App {
    window: Option<&'static Window>,
    surface: Option<Surface<&'static Window, &'static Window>>,
    surface_size: Option<(u32, u32)>,
    image: Option<image::DynamicImage>,
    gif_frames: Vec<image::DynamicImage>,
    gif_delays: Vec<Duration>,
    gif_frame_index: usize,
    gif_next_frame: Option<Instant>,
    images: Vec<PathBuf>,
    image_index: usize,
    zoom: f64,
    rotation: i32,
    verbose: bool,
    fullscreen: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            surface_size: None,
            image: None,
            gif_frames: Vec::new(),
            gif_delays: Vec::new(),
            gif_frame_index: 0,
            gif_next_frame: None,
            images: Vec::new(),
            image_index: 0,
            zoom: 1.0,
            rotation: 0,
            verbose: false,
            fullscreen: false,
        }
    }

    fn toggle_fullscreen(&mut self) {
        let Some(window) = self.window else {
            return;
        };

        self.fullscreen = !self.fullscreen;
        window.set_fullscreen(if self.fullscreen {
            Some(winit::window::Fullscreen::Borderless(None))
        } else {
            None
        });

        if self.verbose {
            debug_log!("[DEBUG] fullscreen: {}", self.fullscreen);
        }

        window.request_redraw();
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, event: &KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }

        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };

        if self.verbose {
            debug_log!("[DEBUG] key pressed: {code:?}");
        }

        match code {
            KeyCode::Escape => {
                if self.fullscreen {
                    self.toggle_fullscreen();
                } else {
                    event_loop.exit();
                }
            }
            KeyCode::Space => self.toggle_fullscreen(),
            KeyCode::Digit0 | KeyCode::Numpad0 => {
                self.reset_view();
            }
            KeyCode::Equal | KeyCode::NumpadAdd => {
                self.zoom_by(1.25);
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                self.zoom_by(0.8);
            }
            KeyCode::KeyR => {
                self.rotate(90);
            }
            KeyCode::KeyL => {
                self.rotate(-90);
            }
            KeyCode::KeyS => {
                self.save_image();
            }
            KeyCode::ArrowRight | KeyCode::ArrowDown | KeyCode::PageDown => {
                self.next_image();
            }
            KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::PageUp => {
                self.prev_image();
            }
            KeyCode::Home => {
                if !self.images.is_empty() {
                    self.image_index = 0;
                    let path = self.images[0].clone();
                    self.load_image(path.to_string_lossy().as_ref());
                    if let Some(window) = self.window {
                        window.request_redraw();
                    }
                }
            }
            KeyCode::End => {
                if !self.images.is_empty() {
                    self.image_index = self.images.len() - 1;
                    let path = self.images[self.image_index].clone();
                    self.load_image(path.to_string_lossy().as_ref());
                    if let Some(window) = self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }

    fn load_image(&mut self, path: &str) {
        if self.verbose {
            debug_log!("[DEBUG] loading image: {path}");
        }

        self.gif_frames.clear();
        self.gif_delays.clear();
        self.gif_frame_index = 0;
        self.gif_next_frame = None;
        self.rotation = 0;

        if std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("gif"))
            .unwrap_or(false)
        {
            let file = std::fs::File::open(path)
                .unwrap_or_else(|e| panic!("failed to open image '{path}': {e}"));
            let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(file))
                .unwrap_or_else(|e| panic!("failed to decode gif '{path}': {e}"));

            let frames = decoder
                .into_frames()
                .collect_frames()
                .unwrap_or_else(|e| panic!("failed to decode gif frames '{path}': {e}"));

            if frames.is_empty() {
                panic!("gif '{path}' contains no frames");
            }

            for frame in frames {
                let delay = frame.delay().numer_denom_ms();
                let delay = Duration::from_millis(
                    ((delay.0 as u64) + (delay.1 as u64) - 1) / delay.1 as u64,
                )
                .max(Duration::from_millis(10));
                self.gif_delays.push(delay);
                self.gif_frames
                    .push(image::DynamicImage::ImageRgba8(frame.into_buffer()));
            }

            self.image = Some(self.gif_frames[0].clone());
            self.gif_next_frame = Some(Instant::now() + self.gif_delays[0]);

            if self.verbose {
                debug_log!("[DEBUG] decoded gif: {} frames", self.gif_frames.len());
            }
            return;
        }

        let image = image::ImageReader::open(path)
            .unwrap_or_else(|e| panic!("failed to open image '{path}': {e}"))
            .decode()
            .unwrap_or_else(|e| panic!("failed to decode image '{path}': {e}"));

        if self.verbose {
            debug_log!(
                "[DEBUG] decoded image: {}x{}, color={:?}",
                image.width(),
                image.height(),
                image.color()
            );
        }

        self.image = Some(image);
    }

    fn update_gif(&mut self) {
        if self.gif_frames.len() <= 1 {
            return;
        }

        let Some(next_frame) = self.gif_next_frame else {
            return;
        };

        let now = Instant::now();
        if now < next_frame {
            return;
        }

        let mut deadline = next_frame;
        loop {
            self.gif_frame_index = (self.gif_frame_index + 1) % self.gif_frames.len();
            deadline += self.gif_delays[self.gif_frame_index];

            if deadline > now {
                break;
            }
        }

        self.image = Some(self.gif_frames[self.gif_frame_index].clone());
        self.gif_next_frame = Some(deadline);

        if self.verbose {
            let remaining_ms = deadline.saturating_duration_since(now).as_millis();
            debug_log!(
                "[DEBUG] gif frame: {}/{}, next in {}ms",
                self.gif_frame_index + 1,
                self.gif_frames.len(),
                remaining_ms
            );
        }
    }

    fn build_gallery(&mut self, path: &str) {
        let Some(parent) = std::path::Path::new(path).parent() else {
            return;
        };

        let extensions = ["jpg", "jpeg", "png", "bmp", "gif", "webp", "tga"];

        let mut images: Vec<PathBuf> = std::fs::read_dir(parent)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| extensions.contains(&ext.to_ascii_lowercase().as_str()))
                        .unwrap_or(false)
            })
            .collect();

        images.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .cmp(
                    &b.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase(),
                )
        });

        self.images = images;
        self.image_index = self
            .images
            .iter()
            .position(|image| image == std::path::Path::new(path))
            .unwrap_or(0);

        if self.verbose {
            debug_log!(
                "[DEBUG] gallery: {} images, index={}",
                self.images.len(),
                self.image_index
            );
        }
    }

    fn next_image(&mut self) {
        if self.images.is_empty() {
            return;
        }

        self.image_index = (self.image_index + 1) % self.images.len();
        let path = self.images[self.image_index].clone();
        self.load_image(path.to_string_lossy().as_ref());

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn prev_image(&mut self) {
        if self.images.is_empty() {
            return;
        }

        self.image_index = if self.image_index == 0 {
            self.images.len() - 1
        } else {
            self.image_index - 1
        };

        let path = self.images[self.image_index].clone();
        self.load_image(path.to_string_lossy().as_ref());

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.rotation = 0;
        if self.verbose {
            debug_log!("[DEBUG] view reset: zoom=100%, rotation=0");
        }
        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn rotate(&mut self, degrees: i32) {
        self.rotation = (self.rotation + degrees).rem_euclid(360);

        if self.verbose {
            debug_log!("[DEBUG] rotation: {} degrees", self.rotation);
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn save_image(&mut self) {
        let Some(image) = self.image.as_ref() else {
            return;
        };

        let Some(source) = self.images.get(self.image_index) else {
            return;
        };

        let Some(stem) = source.file_stem().and_then(|name| name.to_str()) else {
            return;
        };
        let Some(parent) = source.parent() else {
            return;
        };

        let output = parent.join(format!("{stem}_edited.png"));
        let image = match self.rotation {
            90 => image.rotate90(),
            180 => image.rotate180(),
            270 => image.rotate270(),
            _ => image.clone(),
        };

        if let Err(error) = image.save(&output) {
            if self.verbose {
                debug_log!("[DEBUG] save failed: {}: {}", output.display(), error);
            }
            return;
        }

        if self.verbose {
            debug_log!("[DEBUG] saved image: {}", output.display());
        }
    }

    fn zoom_by(&mut self, factor: f64) {
        self.zoom = (self.zoom * factor).clamp(0.05, 20.0);

        if self.verbose {
            debug_log!("[DEBUG] zoom: {:.0}%", self.zoom * 100.0);
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn render(&mut self) {
        let render_start = Instant::now();
        if self.verbose {
            debug_log!("[DEBUG] render: begin");
        }
        self.update_gif();

        let Some(window) = self.window else {
            return;
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let Some(image) = self.image.as_ref() else {
            return;
        };

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        let current_size = (size.width, size.height);
        if self.surface_size != Some(current_size) {
            surface
                .resize(
                    NonZeroU32::new(size.width).unwrap(),
                    NonZeroU32::new(size.height).unwrap(),
                )
                .expect("failed to resize surface");
            self.surface_size = Some(current_size);

            if self.verbose {
                debug_log!("[DEBUG] surface resized: {}x{}", size.width, size.height);
            }
        }

        let mut buffer = surface.buffer_mut().expect("failed to get framebuffer");

        for pixel in buffer.iter_mut() {
            *pixel = 0x202020;
        }

        let image = match self.rotation {
            90 => image.rotate90(),
            180 => image.rotate180(),
            270 => image.rotate270(),
            _ => image.clone(),
        };

        let rgba = image.to_rgba8();
        let src_w = rgba.width();
        let src_h = rgba.height();

        let fit_scale = (size.width as f64 / src_w as f64).min(size.height as f64 / src_h as f64);
        let scale = fit_scale * self.zoom;

        let dst_w = (src_w as f64 * scale).round() as u32;
        let dst_h = (src_h as f64 * scale).round() as u32;
        if dst_w == 0 || dst_h == 0 {
            return;
        }

        let filter = if self.gif_frames.len() > 1 {
            image::imageops::FilterType::Nearest
        } else if scale < 1.0 {
            image::imageops::FilterType::Lanczos3
        } else {
            image::imageops::FilterType::CatmullRom
        };
        let resize_start = Instant::now();
        let resized = if dst_w == src_w && dst_h == src_h {
            rgba
        } else {
            image::imageops::resize(&rgba, dst_w, dst_h, filter)
        };
        let resize_time = resize_start.elapsed();

        if self.verbose {
            debug_log!("[DEBUG] render: resize complete");
        }

        let offset_x = (size.width as i64 - dst_w as i64) / 2;
        let offset_y = (size.height as i64 - dst_h as i64) / 2;

        if self.verbose {
            debug_log!(
                "[DEBUG] render: {}x{} -> {}x{}, filter={:?}, offset=({}, {})",
                src_w, src_h, dst_w, dst_h, filter, offset_x, offset_y
            );
        }

        let resized_width = resized.width() as usize;
        let resized_height = resized.height() as usize;
        let resized_raw = resized.as_raw();

        let copy_start = Instant::now();
        for y in 0..dst_h {
            let dest_y = offset_y + y as i64;
            if dest_y < 0 || dest_y >= size.height as i64 || y >= resized_height as u32 {
                continue;
            }

            let src_y = y as usize;
            for x in 0..dst_w {
                let dest_x = offset_x + x as i64;
                if dest_x < 0 || dest_x >= size.width as i64 || x >= resized_width as u32 {
                    continue;
                }

                let src_index = (src_y * resized_width + x as usize) * 4;
                let pixel = &resized_raw[src_index..src_index + 4];

                let index = (dest_y as u32 * size.width + dest_x as u32) as usize;
                buffer[index] =
                    ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32;
            }
        }
        let copy_time = copy_start.elapsed();

        if self.verbose {
            debug_log!("[DEBUG] render: copy complete");
        }

        let present_start = Instant::now();
        buffer.present().expect("failed to present framebuffer");
        let present_time = present_start.elapsed();

        if self.verbose {
            debug_log!(
                "[DEBUG] render_time: {}ms, resize={}ms, copy={}ms, present={}ms",
                render_start.elapsed().as_millis(),
                resize_time.as_millis(),
                copy_time.as_millis(),
                present_time.as_millis()
            );
        }
    }
}

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.gif_frames.len() <= 1 {
            return;
        }

        let Some(next_frame) = self.gif_next_frame else {
            return;
        };

        let now = Instant::now();
        if now >= next_frame {
            if let Some(window) = self.window {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::Wait);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Box::leak(Box::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("XemAnh2")
                        .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0)),
                )
                .expect("failed to create window"),
        ));

        let window_ref: &'static Window = &*window;

        let context = Context::new(window_ref).expect("failed to create softbuffer context");

        let surface =
            Surface::new(&context, window_ref).expect("failed to create softbuffer surface");

        self.window = Some(window_ref);
        self.surface = Some(surface);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                self.render();
            }

            WindowEvent::Resized(_) => {
                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_key(event_loop, &event);
            }

            WindowEvent::DroppedFile(path) => {
                if let Some(path) = path.to_str() {
                    debug_log!("[DEBUG] dropped file: {}", path);
                    self.build_gallery(path);
                    self.load_image(path);
                    self.zoom = 1.0;

                    if let Some(window) = self.window {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(position) => position.y / 40.0,
                };

                if amount > 0.0 {
                    self.zoom_by(1.1_f64.powf(amount));
                } else if amount < 0.0 {
                    self.zoom_by(1.1_f64.powf(amount));
                }
            }

            _ => {}
        }
    }
}

fn main() {
    let mut verbose = false;
    let mut image_path: Option<String> = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-v" | "--verbose" => verbose = true,
            _ if image_path.is_none() => image_path = Some(arg),
            _ => {
                debug_log!("error: unexpected argument: {arg}");
                std::process::exit(2);
            }
        }
    }

    if verbose {
        init_debug_log(true);
        debug_log!("[DEBUG] verbose logging enabled");
        debug_log!("[DEBUG] image path: {:?}", image_path);
    }

    if image_path.is_none() {
        let home = env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(PathBuf::from);

        if let Some(pictures) = home.map(|home| home.join("Pictures")) {
            let extensions = ["jpg", "jpeg", "png", "bmp", "gif", "webp", "tga"];
            let mut images: Vec<PathBuf> = std::fs::read_dir(&pictures)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| extensions.contains(&ext.to_ascii_lowercase().as_str()))
                            .unwrap_or(false)
                })
                .collect();

            images.sort_by(|a, b| {
                a.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .cmp(
                        &b.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase(),
                    )
            });

            if let Some(path) = images.first() {
                image_path = Some(path.to_string_lossy().into_owned());
            }

            if verbose {
                debug_log!(
                    "[DEBUG] default Pictures: {}, images={}",
                    pictures.display(),
                    images.len()
                );
            }
        }
    }

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new();
    app.verbose = verbose;

    if let Some(path) = image_path {
        app.build_gallery(&path);
        app.load_image(&path);
    }

    event_loop.run_app(&mut app).expect("event loop failed");
}
