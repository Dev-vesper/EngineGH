use std::collections::HashSet;

use glam::Vec2;
use winit::keyboard::KeyCode;

/// Per-frame input snapshot. Written by the event loop, read by the app.
#[derive(Default)]
pub struct Input {
    down: HashSet<KeyCode>,
    mouse_delta: Vec2,
    cursor_grabbed: bool,
}

impl Input {
    #[inline]
    pub fn is_down(&self, key: KeyCode) -> bool {
        self.down.contains(&key)
    }

    /// Accumulated raw mouse motion this frame (pixels). Zero unless the cursor is grabbed.
    #[inline]
    pub fn mouse_delta(&self) -> Vec2 {
        self.mouse_delta
    }

    #[inline]
    pub fn cursor_grabbed(&self) -> bool {
        self.cursor_grabbed
    }

    /// Returns (+1, 0, -1) from a positive/negative key pair.
    #[inline]
    pub fn axis(&self, positive: KeyCode, negative: KeyCode) -> f32 {
        f32::from(self.is_down(positive)) - f32::from(self.is_down(negative))
    }

    pub(crate) fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.down.insert(key);
        } else {
            self.down.remove(&key);
        }
    }

    pub(crate) fn add_mouse_delta(&mut self, dx: f64, dy: f64) {
        if self.cursor_grabbed {
            self.mouse_delta += Vec2::new(dx as f32, dy as f32);
        }
    }

    pub(crate) fn set_cursor_grabbed(&mut self, grabbed: bool) {
        self.cursor_grabbed = grabbed;
        self.mouse_delta = Vec2::ZERO;
    }

    pub(crate) fn clear_all(&mut self) {
        self.down.clear();
        self.mouse_delta = Vec2::ZERO;
    }

    pub(crate) fn end_frame(&mut self) {
        self.mouse_delta = Vec2::ZERO;
    }
}
