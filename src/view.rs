use crate::{debug_log, App};

impl App {
    pub(crate) fn reset_view(&mut self) {
        self.target_zoom = 1.0;
        self.rotation = 0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.zoom_anchor = None;

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

    fn render_scale(&self, zoom: f64) -> Option<f64> {
        let window = self.window?;
        let image = self.image.as_ref()?;

        let scale_factor = window.scale_factor();
        let size = window.inner_size();

        let (src_w, src_h) = match self.rotation.rem_euclid(360) {
            90 | 270 => (image.height(), image.width()),
            _ => (image.width(), image.height()),
        };

        if src_w == 0 || src_h == 0 {
            return None;
        }

        let logical_width = size.width as f64 / scale_factor;
        let logical_height = size.height as f64 / scale_factor;

        let fit_scale = (logical_width / src_w as f64)
            .min(logical_height / src_h as f64);

        Some(if (zoom - 1.0).abs() < f64::EPSILON {
            fit_scale.min(1.0)
        } else {
            zoom
        })
    }

    pub(crate) fn update_zoom_anchor(&mut self) {
        let Some((cursor_x, cursor_y, image_x, image_y)) = self.zoom_anchor else {
            return;
        };

        let Some(window) = self.window else {
            return;
        };

        let Some(scale) = self.render_scale(self.zoom) else {
            return;
        };

        let scale_factor = window.scale_factor();
        let size = window.inner_size();

        let center_x = size.width as f64 / scale_factor / 2.0;
        let center_y = size.height as f64 / scale_factor / 2.0;

        self.pan_x = cursor_x - center_x - image_x * scale;
        self.pan_y = cursor_y - center_y - image_y * scale;
    }

    pub(crate) fn zoom_by(&mut self, factor: f64) {
        self.zoom_anchor = None;
        let old_zoom = self.target_zoom;
        let new_zoom = (old_zoom * factor).clamp(0.05, 20.0);

        if let (Some(window), Some(image)) = (self.window, self.image.as_ref()) {
            let scale_factor = window.scale_factor();

            let (src_w, src_h) = match self.rotation.rem_euclid(360) {
                90 | 270 => (image.height(), image.width()),
                _ => (image.width(), image.height()),
            };

            let size = window.inner_size();
            let fit_scale = if src_w == 0 || src_h == 0 {
                1.0
            } else {
                (size.width as f64 / src_w as f64)
                    .min(size.height as f64 / src_h as f64)
            };

            let old_scale = if (old_zoom - 1.0).abs() < f64::EPSILON {
                fit_scale.min(scale_factor)
            } else {
                old_zoom * scale_factor
            };

            let new_scale = if (new_zoom - 1.0).abs() < f64::EPSILON {
                fit_scale.min(scale_factor)
            } else {
                new_zoom * scale_factor
            };

            if old_scale > 0.0 && new_scale > 0.0 {
                let logical_width = size.width as f64 / scale_factor;
                let logical_height = size.height as f64 / scale_factor;
                let center_x = logical_width / 2.0;
                let center_y = logical_height / 2.0;

                let cursor_dx = self.cursor_x - center_x;
                let cursor_dy = self.cursor_y - center_y;

                let scale_ratio = new_scale / old_scale;

                self.pan_x =
                    cursor_dx - (cursor_dx - self.pan_x) * scale_ratio;
                self.pan_y =
                    cursor_dy - (cursor_dy - self.pan_y) * scale_ratio;
            }
        }

        self.target_zoom = new_zoom;

        if self.verbose {
            debug_log(format!(
                "[DEBUG] zoom target: {:.0}%, pan=({:.1}, {:.1})",
                self.target_zoom * 100.0,
                self.pan_x,
                self.pan_y
            ));
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    pub(crate) fn zoom_at_cursor(&mut self, factor: f64) {
        let old_scale = match self.render_scale(self.zoom) {
            Some(scale) if scale > 0.0 => scale,
            _ => {
                self.zoom_by(factor);
                return;
            }
        };

        let Some(window) = self.window else {
            self.zoom_by(factor);
            return;
        };

        let scale_factor = window.scale_factor();
        let size = window.inner_size();

        let center_x = size.width as f64 / scale_factor / 2.0;
        let center_y = size.height as f64 / scale_factor / 2.0;

        let image_x = (self.cursor_x - center_x - self.pan_x) / old_scale;
        let image_y = (self.cursor_y - center_y - self.pan_y) / old_scale;

        self.zoom_anchor = Some((
            self.cursor_x,
            self.cursor_y,
            image_x,
            image_y,
        ));

        self.target_zoom = (self.target_zoom * factor).clamp(0.05, 20.0);

        if self.verbose {
            debug_log(format!(
                "[DEBUG] cursor zoom target: {:.0}%, anchor=({:.1}, {:.1})",
                self.target_zoom * 100.0,
                image_x,
                image_y
            ));
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }
}
