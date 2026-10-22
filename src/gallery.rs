use std::path::{Path, PathBuf};

use crate::{debug_log, App};

impl App {
    pub(crate) fn build_gallery(&mut self, path: &str) {
        let Some(parent) = Path::new(path).parent() else {
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
            .position(|image| image == Path::new(path))
            .unwrap_or(0);

        if self.verbose {
            debug_log(format!(
                "[DEBUG] gallery: {} images, index={}",
                self.images.len(),
                self.image_index
            ));
        }
    }

    pub(crate) fn next_image(&mut self) {
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

    pub(crate) fn prev_image(&mut self) {
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
}
