#![cfg_attr(windows, windows_subsystem = "windows")]

use softbuffer::{Context, Surface};
use std::{env, fs::OpenOptions, io::Write, num::NonZeroU32, path::PathBuf, sync::Mutex};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

static DEBUG_LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

fn init_debug_log(enabled: bool) {
    if !enabled {
        return;
    }

    let Some(local_app_data) = env::var_os("LOCALAPPDATA") else {
        return;
    };

    let dir = PathBuf::from(local_app_data).join("XemAnh2");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }

    let path = dir.join("xemanh2.log");
    if let Ok(file) = OpenOptions::new().create(true).append(true).open(path) {
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
    image: Option<image::DynamicImage>,
    images: Vec<PathBuf>,
    image_index: usize,
    zoom: f64,
    verbose: bool,
    fullscreen: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            image: None,
            images: Vec::new(),
            image_index: 0,
            zoom: 1.0,
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
            debug_log!("[DEBUG] key: {code:?}");
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
        if self.verbose {
            debug_log!("[DEBUG] zoom reset: 100%");
        }
        if let Some(window) = self.window {
            window.request_redraw();
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

        surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .expect("failed to resize surface");

        let mut buffer = surface.buffer_mut().expect("failed to get framebuffer");

        for pixel in buffer.iter_mut() {
            *pixel = 0x202020;
        }

        let rgba = image.to_rgba8();
        let src_w = rgba.width();
        let src_h = rgba.height();

        let fit_scale = (size.width as f64 / src_w as f64).min(size.height as f64 / src_h as f64);
        let scale = fit_scale * self.zoom;

        let dst_w = (src_w as f64 * scale).round() as u32;
        let dst_h = (src_h as f64 * scale).round() as u32;
        let offset_x = (size.width as i64 - dst_w as i64) / 2;
        let offset_y = (size.height as i64 - dst_h as i64) / 2;

        if self.verbose {
            debug_log!(
                "[DEBUG] render: {}x{} -> {}x{}, offset=({}, {})",
                src_w, src_h, dst_w, dst_h, offset_x, offset_y
            );
        }

        for y in 0..dst_h {
            let dest_y = offset_y + y as i64;
            if dest_y < 0 || dest_y >= size.height as i64 {
                continue;
            }

            let sy = (y as u64 * src_h as u64 / dst_h as u64) as u32;

            for x in 0..dst_w {
                let dest_x = offset_x + x as i64;
                if dest_x < 0 || dest_x >= size.width as i64 {
                    continue;
                }

                let sx = (x as u64 * src_w as u64 / dst_w as u64) as u32;
                let pixel = rgba.get_pixel(sx, sy);

                let index = (dest_y as u32 * size.width + dest_x as u32) as usize;
                buffer[index] =
                    ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32;
            }
        }

        buffer.present().expect("failed to present framebuffer");
    }
}

impl ApplicationHandler for App {
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
