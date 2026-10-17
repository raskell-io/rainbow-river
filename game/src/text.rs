//! The text box: two lines of text on BG0 over a translucent panel on BG1,
//! typed out one character per frame, A or B to advance.

use crate::assets::{bank, bg_tile};
use crate::input::{Input, btn};
use wasm_ad::addr::SCREEN_H;
use wasm_ad::{ScreenEntry, dispcnt, reg, screen_block};

pub const LINE_CHARS: usize = 26;
pub const BOX_COLS: usize = 30;
pub const BOX_ROWS: usize = 5;
/// Map row of the box's top edge.
const BOX_ROW: usize = SCREEN_H / 8 - BOX_ROWS;
const TEXT_COL: usize = 2;
/// Screen blocks holding the text (BG0) and panel (BG1) maps.
pub const TEXT_BLOCK: u16 = 28;
pub const PANEL_BLOCK: u16 = 29;

pub struct TextBox {
    pages: Vec<[String; 2]>,
    page: usize,
    shown: usize,
    frame: u32,
}

impl TextBox {
    /// Word-wraps `msg` into pages. `\n` forces a line break.
    pub fn new(msg: &str) -> Self {
        let lines = wrap(msg);
        let pages = lines
            .chunks(2)
            .map(|c| [c[0].clone(), c.get(1).cloned().unwrap_or_default()])
            .collect();
        Self::from_pages(pages)
    }

    pub fn from_pages(pages: Vec<[String; 2]>) -> Self {
        Self { pages, page: 0, shown: 0, frame: 0 }
    }

    fn page_len(&self) -> usize {
        let [a, b] = &self.pages[self.page];
        a.len() + b.len()
    }

    /// Returns true once the last page has been dismissed.
    pub fn update(&mut self, input: &Input) -> bool {
        self.frame += 1;
        let advance = input.pressed(btn::A) || input.pressed(btn::B);
        if self.shown < self.page_len() {
            self.shown = if advance { self.page_len() } else { self.shown + 1 };
            return false;
        }
        if advance {
            if self.page + 1 == self.pages.len() {
                return true;
            }
            self.page += 1;
            self.shown = 0;
        }
        false
    }

    pub fn draw(&self) {
        show_panel();
        let [a, b] = &self.pages[self.page];
        put_text(TEXT_COL, 1, &a[..self.shown.min(a.len())]);
        put_text(TEXT_COL, 3, &b[..self.shown.saturating_sub(a.len())]);
        if self.shown == self.page_len() && self.frame / 16 % 2 == 0 {
            put_tile(BOX_COLS - 3, 3, bg_tile::ARROW);
        }
    }
}

/// Shows an empty box: clears the text map, draws the panel and enables both layers.
pub fn show_panel() {
    let last_col = BOX_COLS - 1;
    let last_row = BOX_ROWS - 1;
    let e = |tile| ScreenEntry::new(tile, bank::PANEL);
    screen_block(TEXT_BLOCK as usize).fill(ScreenEntry::EMPTY);
    let panel = screen_block(PANEL_BLOCK as usize);
    for row in 0..BOX_ROWS {
        for col in 0..BOX_COLS {
            panel[(BOX_ROW + row) * 32 + col] = match (col, row) {
                (0, 0) => e(bg_tile::PANEL_CORNER),
                (c, 0) if c == last_col => e(bg_tile::PANEL_CORNER).hflip(),
                (0, r) if r == last_row => e(bg_tile::PANEL_CORNER).vflip(),
                (c, r) if c == last_col && r == last_row => e(bg_tile::PANEL_CORNER).hflip().vflip(),
                (_, 0) => e(bg_tile::PANEL_HEDGE),
                (_, r) if r == last_row => e(bg_tile::PANEL_HEDGE).vflip(),
                (0, _) => e(bg_tile::PANEL_VEDGE),
                (c, _) if c == last_col => e(bg_tile::PANEL_VEDGE).hflip(),
                _ => e(bg_tile::PANEL_FILL),
            };
        }
    }
    reg::DISPCNT.write(reg::DISPCNT.read() | dispcnt::BG0 | dispcnt::BG1);
}

pub fn hide() {
    reg::DISPCNT.write(reg::DISPCNT.read() & !(dispcnt::BG0 | dispcnt::BG1));
}

/// Writes text at a column and row inside the box.
pub fn put_text(col: usize, row: usize, text: &str) {
    for (i, b) in text.bytes().enumerate() {
        put_tile(col + i, row, b as u16);
    }
}

pub fn put_tile(col: usize, row: usize, tile: u16) {
    screen_block(TEXT_BLOCK as usize)[(BOX_ROW + row) * 32 + col] = ScreenEntry::new(tile, bank::TEXT);
}

fn wrap(msg: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for para in msg.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + word.len() > LINE_CHARS {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_on_word_boundaries() {
        assert_eq!(
            wrap("A QUIET MORNING ON THE RIVER BANK.\n42CM"),
            ["A QUIET MORNING ON THE", "RIVER BANK.", "42CM"]
        );
    }

    #[test]
    fn pages_hold_two_lines() {
        let tb = TextBox::new("ONE\nTWO\nTHREE");
        assert_eq!(tb.pages.len(), 2);
        assert_eq!(tb.pages[1], ["THREE".to_string(), String::new()]);
    }
}
