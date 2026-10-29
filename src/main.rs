#![cfg_attr(windows, windows_subsystem = "windows")]

use softbuffer::{Context, Surface};
use std::{
    env,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

mod renderer;
mod animation;
mod image_io;
mod gallery;
mod view;
mod input;

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
    render_rgba: Option<image::RgbaImage>,
    render_rgba_rotation: i32,
    render_scaled: Option<image::RgbaImage>,
    render_scaled_size: Option<(u32, u32)>,
    render_scaled_filter: Option<image::imageops::FilterType>,
    gif_frames: Vec<image::DynamicImage>,
    gif_delays: Vec<Duration>,
    gif_frame_index: usize,
    gif_next_frame: Option<Instant>,
    images: Vec<PathBuf>,
    image_index: usize,
    zoom: f64,
    target_zoom: f64,
    rotation: i32,
    pan_x: f64,
    pan_y: f64,
    zoom_anchor: Option<(f64, f64, f64, f64)>,
    dragging: bool,
    cursor_x: f64,
    cursor_y: f64,
    drag_last_x: f64,
    drag_last_y: f64,
    verbose: bool,
    fullscreen: bool,
    show_info: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            surface_size: None,
            image: None,
        render_rgba: None,
        render_rgba_rotation: 0,
        render_scaled: None,
        render_scaled_size: None,
        render_scaled_filter: None,
        gif_frames: Vec::new(),
            gif_delays: Vec::new(),
            gif_frame_index: 0,
            gif_next_frame: None,
            images: Vec::new(),
            image_index: 0,
            zoom: 1.0,
            target_zoom: 1.0,
            rotation: 0,
            pan_x: 0.0,
            pan_y: 0.0,
            zoom_anchor: None,
            dragging: false,
            cursor_x: 0.0,
            cursor_y: 0.0,
            drag_last_x: 0.0,
            drag_last_y: 0.0,
            verbose: false,
            fullscreen: false,
            show_info: false,
        }
    }

    pub(crate) fn update_window_title(&self) {
        let Some(window) = self.window else {
            return;
        };

        if !self.show_info {
            window.set_title("XemAnh2");
            return;
        }

        let Some(path) = self.images.get(self.image_index) else {
            window.set_title("XemAnh2");
            return;
        };

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown");

        let Some(image) = self.image.as_ref() else {
            window.set_title(name);
            return;
        };

        let format = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("?")
            .to_uppercase();

        let size = std::fs::metadata(path)
            .map(|metadata| {
                let bytes = metadata.len();
                if bytes < 1024 {
                    format!("{bytes} B")
                } else if bytes < 1024 * 1024 {
                    format!("{:.1} KB", bytes as f64 / 1024.0)
                } else {
                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                }
            })
            .unwrap_or_else(|_| "?".to_string());

        let position = if self.images.is_empty() {
            String::new()
        } else {
            format!(" | {} / {}", self.image_index + 1, self.images.len())
        };

        window.set_title(&format!(
            "{name} | {}x{} | {format} | {size}{position} - XemAnh2",
            image.width(),
            image.height()
        ));
    }

}

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        const ZOOM_EPSILON: f64 = 0.001;
        const ZOOM_SMOOTHING: f64 = 0.28;

        if (self.zoom - self.target_zoom).abs() > ZOOM_EPSILON {
            self.zoom += (self.target_zoom - self.zoom) * ZOOM_SMOOTHING;
            self.update_zoom_anchor();

            if let Some(window) = self.window {
                window.request_redraw();
            }

            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(16),
            ));
            return;
        }

        self.zoom = self.target_zoom;
        self.update_zoom_anchor();
        self.zoom_anchor = None;

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
                    self.target_zoom = 1.0;
                    self.pan_x = 0.0;
                    self.pan_y = 0.0;
                    self.zoom_anchor = None;

                    if let Some(window) = self.window {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let scale_factor = self.window.map(|window| window.scale_factor()).unwrap_or(1.0);
                let x = position.x / scale_factor;
                let y = position.y / scale_factor;

                if self.dragging {
                    self.pan_x += x - self.drag_last_x;
                    self.pan_y += y - self.drag_last_y;
                    self.drag_last_x = x;
                    self.drag_last_y = y;

                    if let Some(window) = self.window {
                        window.request_redraw();
                    }
                }

                self.cursor_x = x;
                self.cursor_y = y;
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Left {
                    match state {
                        winit::event::ElementState::Pressed => {
                            self.dragging = true;
                            self.zoom_anchor = None;
                            self.drag_last_x = self.cursor_x;
                            self.drag_last_y = self.cursor_y;
                        }
                        winit::event::ElementState::Released => {
                            self.dragging = false;
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(position) => position.y / 40.0,
                };

                if amount != 0.0 {
                    self.zoom_at_cursor(1.1_f64.powf(amount));
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
