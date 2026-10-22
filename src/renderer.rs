use image::DynamicImage;
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
        let Some(image) = self.image.as_ref() else {
            return;
        };

        render_image(
            window,
            surface,
            image,
            self.rotation,
            self.zoom,
            self.gif_frames.len() > 1,
            &mut self.surface_size,
            self.verbose,
        );
    }
}

fn render_image(
    window: &Window,
    surface: &mut Surface<&Window, &Window>,
    image: &DynamicImage,
    rotation: i32,
    zoom: f64,
    is_gif: bool,
    surface_size: &mut Option<(u32, u32)>,
    verbose: bool,
) {
    let render_start = Instant::now();
    let size = window.inner_size();
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

    let image = match rotation {
        90 => image.rotate90(),
        180 => image.rotate180(),
        270 => image.rotate270(),
        _ => image.clone(),
    };

    let rgba = image.to_rgba8();
    let src_w = rgba.width();
    let src_h = rgba.height();

    let fit_scale =
        (size.width as f64 / src_w as f64).min(size.height as f64 / src_h as f64);
    let scale = if is_gif {
        zoom
    } else {
        fit_scale * zoom
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
    let resize_start = Instant::now();
    let resized = if dst_w == src_w && dst_h == src_h {
        rgba
    } else {
        image::imageops::resize(&rgba, dst_w, dst_h, filter)
    };
    let resize_time = resize_start.elapsed();

    if verbose {
        debug_log("[DEBUG] render: resize complete".to_string());
    }

    let offset_x = (size.width as i64 - dst_w as i64) / 2;
    let offset_y = (size.height as i64 - dst_h as i64) / 2;

    if verbose {
        debug_log(format!(
            "[DEBUG] render: {}x{} -> {}x{}, filter={:?}, offset=({}, {})",
            src_w,
            src_h,
            dst_w,
            dst_h,
            filter,
            offset_x,
            offset_y
        ));
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

    if verbose {
        debug_log("[DEBUG] render: copy complete".to_string());
    }

    let present_start = Instant::now();
    buffer.present().expect("failed to present framebuffer");
    let present_time = present_start.elapsed();

    if verbose {
        debug_log(format!(
            "[DEBUG] render_time: {}ms, resize={}ms, copy={}ms, present={}ms",
            render_start.elapsed().as_millis(),
            resize_time.as_millis(),
            copy_time.as_millis(),
            present_time.as_millis()
        ));
    }
}
