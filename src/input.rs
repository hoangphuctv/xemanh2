use winit::{
    event::{ElementState, KeyEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{debug_log, App};

impl App {
    pub(crate) fn toggle_fullscreen(&mut self) {
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
            debug_log(format!("[DEBUG] fullscreen: {}", self.fullscreen));
        }

        window.request_redraw();
    }

    pub(crate) fn handle_key(&mut self, event_loop: &ActiveEventLoop, event: &KeyEvent) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }

        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };

        if self.verbose {
            debug_log(format!("[DEBUG] key pressed: {code:?}"));
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
            KeyCode::KeyI => {
                self.show_info = !self.show_info;
                self.update_window_title();
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
}
