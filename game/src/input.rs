//! The eight Game Boy buttons, delivered by the host as one bitmask per frame.

#[allow(dead_code)]
pub mod btn {
    pub const RIGHT: u8 = 1 << 0;
    pub const LEFT: u8 = 1 << 1;
    pub const UP: u8 = 1 << 2;
    pub const DOWN: u8 = 1 << 3;
    pub const A: u8 = 1 << 4;
    pub const B: u8 = 1 << 5;
    pub const SELECT: u8 = 1 << 6;
    pub const START: u8 = 1 << 7;
}

#[derive(Default)]
pub struct Input {
    held: u8,
    prev: u8,
}

impl Input {
    pub fn update(&mut self, raw: u8) {
        self.prev = self.held;
        self.held = raw;
    }

    pub fn held(&self, b: u8) -> bool {
        self.held & b != 0
    }

    /// True only on the frame the button went down.
    pub fn pressed(&self, b: u8) -> bool {
        self.held & !self.prev & b != 0
    }
}
