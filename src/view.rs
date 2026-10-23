use crate::{debug_log, App};

impl App {
    pub(crate) fn reset_view(&mut self) {
        self.target_zoom = 1.0;
        self.rotation = 0;

        if self.verbose {
            debug_log("[DEBUG] view reset: zoom=100%, rotation=0".to_string());
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn rotate(&mut self, degrees: i32) {
        self.rotation = (self.rotation + degrees).rem_euclid(360);

        if self.verbose {
            debug_log(format!(
                "[DEBUG] rotation: {} degrees",
                self.rotation
            ));
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn zoom_by(&mut self, factor: f64) {
        self.target_zoom = (self.target_zoom * factor).clamp(0.05, 20.0);

        if self.verbose {
            debug_log(format!(
                "[DEBUG] zoom target: {:.0}%",
                self.target_zoom * 100.0
            ));
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }
}
