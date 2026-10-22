use std::time::Instant;

use crate::{debug_log, App};

impl App {
    pub(crate) fn update_gif(&mut self) {
        if self.gif_frames.len() <= 1 {
            return;
        }

        let Some(next_frame) = self.gif_next_frame else {
            return;
        };

        let now = Instant::now();
        if now < next_frame {
            return;
        }

        let mut deadline = next_frame;
        loop {
            self.gif_frame_index = (self.gif_frame_index + 1) % self.gif_frames.len();
            deadline += self.gif_delays[self.gif_frame_index];

            if deadline > now {
                break;
            }
        }

        self.image = Some(self.gif_frames[self.gif_frame_index].clone());
        self.gif_next_frame = Some(deadline);

        if self.verbose {
            let remaining_ms = deadline.saturating_duration_since(now).as_millis();
            debug_log(format!(
                "[DEBUG] gif frame: {}/{}, next in {}ms",
                self.gif_frame_index + 1,
                self.gif_frames.len(),
                remaining_ms
            ));
        }
    }
}
