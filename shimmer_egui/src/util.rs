use eframe::egui;
use std::time::{Duration, Instant};

/// Returns the width and height of a character in a given style, in points.
pub fn character_dimensions(ui: &egui::Ui, style: egui::TextStyle, c: char) -> (f32, f32) {
    let font_id = style.resolve(ui.style());
    let width = ui.fonts(|f| f.glyph_width(&font_id, c));
    let height = ui.text_style_height(&style);

    (width, height)
}

pub struct Timer {
    elapsed: Duration,
    resumed_at: Instant,
    running: bool,
    scale: f64,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            resumed_at: Instant::now(),
            running: false,
            scale: 1.0,
        }
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn set_scale(&mut self, value: f64) {
        if self.running {
            self.pause();
            self.scale = value;
            self.resume();
        } else {
            self.scale = value;
        }
    }

    pub fn resume(&mut self) {
        if !self.running {
            self.resumed_at = Instant::now();
            self.running = true;
        }
    }

    pub fn pause(&mut self) {
        if self.running {
            self.elapsed += self.resumed_at.elapsed().mul_f64(self.scale);
            self.running = false;
        }
    }

    pub fn elapsed(&self) -> Duration {
        if self.running {
            self.elapsed + self.resumed_at.elapsed().mul_f64(self.scale)
        } else {
            self.elapsed
        }
    }
}
