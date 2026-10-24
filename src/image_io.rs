use image::AnimationDecoder;
use std::{
    fs::File,
    io::BufReader,
    path::Path,
    time::{Duration, Instant},
};

use crate::{debug_log, App};

impl App {
    pub(crate) fn load_image(&mut self, path: &str) {
        if self.verbose {
            debug_log(format!("[DEBUG] loading image: {path}"));
        }

        self.gif_frames.clear();
        self.gif_delays.clear();
        self.gif_frame_index = 0;
        self.gif_next_frame = None;
        self.render_rgba = None;
        self.render_rgba_rotation = 0;
        self.render_scaled = None;
        self.render_scaled_size = None;
        self.render_scaled_filter = None;
        self.rotation = 0;

        if Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("gif"))
            .unwrap_or(false)
        {
            let file = File::open(path)
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
                debug_log(format!(
                    "[DEBUG] decoded gif: {} frames",
                    self.gif_frames.len()
                ));
            }
            return;
        }

        let image = image::ImageReader::open(path)
            .unwrap_or_else(|e| panic!("failed to open image '{path}': {e}"))
            .decode()
            .unwrap_or_else(|e| panic!("failed to decode image '{path}': {e}"));

        if self.verbose {
            debug_log(format!(
                "[DEBUG] decoded image: {}x{}, color={:?}",
                image.width(),
                image.height(),
                image.color()
            ));
        }

        self.image = Some(image);
    }

    pub(crate) fn save_image(&mut self) {
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
                debug_log(format!(
                    "[DEBUG] save failed: {}: {}",
                    output.display(),
                    error
                ));
            }
            return;
        }

        if self.verbose {
            debug_log(format!("[DEBUG] saved image: {}", output.display()));
        }
    }
}
