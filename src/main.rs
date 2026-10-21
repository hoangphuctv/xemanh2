use softbuffer::{Context, Surface};
use std::{env, num::NonZeroU32};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct App {
    window: Option<&'static Window>,
    surface: Option<Surface<&'static Window, &'static Window>>,
    image: Option<image::DynamicImage>,
    verbose: bool,
    fullscreen: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            image: None,
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
            eprintln!("[DEBUG] fullscreen: {}", self.fullscreen);
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
            eprintln!("[DEBUG] key: {code:?}");
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
                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn load_image(&mut self, path: &str) {
        if self.verbose {
            eprintln!("[DEBUG] loading image: {path}");
        }

        let image = image::ImageReader::open(path)
            .unwrap_or_else(|e| panic!("failed to open image '{path}': {e}"))
            .decode()
            .unwrap_or_else(|e| panic!("failed to decode image '{path}': {e}"));

        if self.verbose {
            eprintln!(
                "[DEBUG] decoded image: {}x{}, color={:?}",
                image.width(),
                image.height(),
                image.color()
            );
        }

        self.image = Some(image);
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

        let scale = (size.width as f64 / src_w as f64).min(size.height as f64 / src_h as f64);

        let dst_w = (src_w as f64 * scale).round() as u32;
        let dst_h = (src_h as f64 * scale).round() as u32;
        let offset_x = (size.width - dst_w) / 2;
        let offset_y = (size.height - dst_h) / 2;

        if self.verbose {
            eprintln!(
                "[DEBUG] render: {}x{} -> {}x{}, offset=({}, {})",
                src_w, src_h, dst_w, dst_h, offset_x, offset_y
            );
        }

        for y in 0..dst_h {
            let sy = (y as u64 * src_h as u64 / dst_h as u64) as u32;

            for x in 0..dst_w {
                let sx = (x as u64 * src_w as u64 / dst_w as u64) as u32;
                let pixel = rgba.get_pixel(sx, sy);

                let index = ((offset_y + y) * size.width + offset_x + x) as usize;
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
                eprintln!("error: unexpected argument: {arg}");
                std::process::exit(2);
            }
        }
    }

    if verbose {
        eprintln!("[DEBUG] verbose logging enabled");
        eprintln!("[DEBUG] image path: {:?}", image_path);
    }

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = App::new();
    app.verbose = verbose;

    if let Some(path) = image_path {
        app.load_image(&path);
    }

    event_loop.run_app(&mut app).expect("event loop failed");
}
