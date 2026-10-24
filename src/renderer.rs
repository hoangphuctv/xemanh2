use image::RgbaImage;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::time::Instant;
use winit::window::Window;

use crate::{debug_log, App};

impl App {
    pub(crate) fn render(&mut self) {
        if self.verbose {
            debug_log("[DEBUG] render: begin".to_string());
        }
        self.update_gif();

        let Some(window) = self.window else {
            return;
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        if self.render_rgba.is_none() || self.render_rgba_rotation != self.rotation {
            let Some(image) = self.image.as_ref() else {
                return;
            };

            let rgba = match self.rotation {
                90 => image.rotate90().to_rgba8(),
                180 => image.rotate180().to_rgba8(),
                270 => image.rotate270().to_rgba8(),
                _ => image.to_rgba8(),
            };

            self.render_rgba = Some(rgba);
            self.render_rgba_rotation = self.rotation;

            if self.verbose {
                debug_log("[DEBUG] render cache: rebuilt".to_string());
            }
        }

        let Some(rgba) = self.render_rgba.as_ref() else {
            return;
        };

        render_image(
            window,
            surface,
            rgba,
            self.zoom,
            self.pan_x,
            self.pan_y,
            (self.zoom - self.target_zoom).abs() > 0.001,
            self.gif_frames.len() > 1,
            &mut self.surface_size,
            &mut self.render_scaled,
            &mut self.render_scaled_size,
            &mut self.render_scaled_filter,
            self.verbose,
        );
    }
}

fn render_image(
    window: &Window,
    surface: &mut Surface<&Window, &Window>,
    image: &RgbaImage,
    zoom: f64,
    pan_x: f64,
    pan_y: f64,
    zoom_animating: bool,
    is_gif: bool,
    surface_size: &mut Option<(u32, u32)>,
    render_scaled: &mut Option<RgbaImage>,
    render_scaled_size: &mut Option<(u32, u32)>,
    render_scaled_filter: &mut Option<image::imageops::FilterType>,
    verbose: bool,
) {
    let render_start = Instant::now();
    let size = window.inner_size();
    let scale_factor = window.scale_factor();
    if size.width == 0 || size.height == 0 {
        return;
    }

    let current_size = (size.width, size.height);
    if *surface_size != Some(current_size) {
        surface
            .resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            )
            .expect("failed to resize surface");
        *surface_size = Some(current_size);

        if verbose {
            debug_log(format!(
                "[DEBUG] surface resized: {}x{}",
                size.width, size.height
            ));
        }
    }

    let mut buffer = surface.buffer_mut().expect("failed to get framebuffer");

    for pixel in buffer.iter_mut() {
        *pixel = 0x202020;
    }

    let rgba = image;
    let src_w = rgba.width();
    let src_h = rgba.height();

    let fit_scale =
        (size.width as f64 / src_w as f64).min(size.height as f64 / src_h as f64);

    // zoom == 1.0 is the untouched/default state:
    // - show images at native size when they fit;
    // - downscale large images only when necessary;
    // - never upscale automatically.
    //
    // Once the user changes zoom, `zoom` is an explicit scale relative
    // to the image's native resolution rather than relative to fit-to-window.
    let scale = if (zoom - 1.0).abs() < f64::EPSILON {
        fit_scale.min(scale_factor)
    } else {
        zoom * scale_factor
    };

    let dst_w = (src_w as f64 * scale).round() as u32;
    let dst_h = (src_h as f64 * scale).round() as u32;
    if dst_w == 0 || dst_h == 0 {
        return;
    }

    let filter = if is_gif {
        image::imageops::FilterType::Nearest
    } else if scale < 1.0 {
        image::imageops::FilterType::Lanczos3
    } else {
        image::imageops::FilterType::CatmullRom
    };

    let offset_x =
        ((size.width as f64 - dst_w as f64) / 2.0 + pan_x * scale_factor).round() as i64;
    let offset_y =
        ((size.height as f64 - dst_h as f64) / 2.0 + pan_y * scale_factor).round() as i64;

    if verbose {
        debug_log(format!(
            "[DEBUG] render: {}x{} -> {}x{}, dpi={:.2}, zoom={:.2}, filter={:?}, offset=({}, {})",
            src_w,
            src_h,
            dst_w,
            dst_h,
            scale_factor,
            zoom,
            filter,
            offset_x,
            offset_y
        ));
    }

    let resize_start = Instant::now();
    let copy_start = Instant::now();

    if zoom_animating {
        // During smooth zoom, avoid allocating/resizing the entire image.
        // Sample only the pixels visible in the framebuffer.
        let raw = image.as_raw();

        let start_x = (-offset_x).max(0) as u32;
        let start_y = (-offset_y).max(0) as u32;
        let end_x = dst_w.min((size.width as i64 - offset_x).max(0) as u32);
        let end_y = dst_h.min((size.height as i64 - offset_y).max(0) as u32);

        for y in start_y..end_y {
            let dest_y = offset_y + y as i64;
            let src_y = ((y as f64) / scale).floor() as u32;
            if src_y >= src_h {
                continue;
            }

            for x in start_x..end_x {
                let dest_x = offset_x + x as i64;
                let src_x = ((x as f64) / scale).floor() as u32;
                if src_x >= src_w {
                    continue;
                }

                let src_index = ((src_y * src_w + src_x) * 4) as usize;
                let pixel = &raw[src_index..src_index + 4];

                let index = (dest_y as u32 * size.width + dest_x as u32) as usize;
                buffer[index] =
                    ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32;
            }
        }

        let resize_time = resize_start.elapsed();
        let copy_time = copy_start.elapsed();

        if verbose {
            debug_log(format!(
                "[DEBUG] render: direct zoom sampling, resize={}ms, copy={}ms",
                resize_time.as_millis(),
                copy_time.as_millis()
            ));
        }
    } else {
        let cache_key = (dst_w, dst_h);
        let cache_valid = *render_scaled_size == Some(cache_key)
            && *render_scaled_filter == Some(filter)
            && render_scaled.is_some();

        if !cache_valid {
            *render_scaled = if dst_w == src_w && dst_h == src_h {
                Some(image.clone())
            } else {
                Some(image::imageops::resize(image, dst_w, dst_h, filter))
            };
            *render_scaled_size = Some(cache_key);
            *render_scaled_filter = Some(filter);
        }

        let resize_time = resize_start.elapsed();
        let resized = render_scaled.as_ref().unwrap();
        let resized_width = resized.width() as usize;
        let resized_raw = resized.as_raw();

        for y in 0..dst_h {
            let dest_y = offset_y + y as i64;
            if dest_y < 0 || dest_y >= size.height as i64 {
                continue;
            }

            for x in 0..dst_w {
                let dest_x = offset_x + x as i64;
                if dest_x < 0 || dest_x >= size.width as i64 {
                    continue;
                }

                let src_index = (y as usize * resized_width + x as usize) * 4;
                let pixel = &resized_raw[src_index..src_index + 4];

                let index = (dest_y as u32 * size.width + dest_x as u32) as usize;
                buffer[index] =
                    ((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32;
            }
        }

        let copy_time = copy_start.elapsed();

        if verbose {
            debug_log(format!(
                "[DEBUG] render: resize {} (cached={}), resize={}ms, copy={}ms",
                if cache_valid { "cache hit" } else { "cache rebuilt" },
                cache_valid,
                resize_time.as_millis(),
                copy_time.as_millis()
            ));
        }
    }

    let present_start = Instant::now();
    buffer.present().expect("failed to present framebuffer");
    let present_time = present_start.elapsed();

    if verbose {
        debug_log(format!(
            "[DEBUG] render_time: {}ms, present={}ms",
            render_start.elapsed().as_millis(),
            present_time.as_millis()
        ));
    }
}
