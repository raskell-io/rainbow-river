//! Builds tiles from ASCII art (placeholders) or converted PNG data.
//!
//! In ASCII art `.` is color 0 (transparent), `+` `x` `#` are 1–3 and the
//! hex digits `1`–`9` `a`–`f` reach all 15 colors of a palette bank. By
//! convention lower indices are lighter, which is also the order `assetc`
//! emits for converted PixelLab art.

use crate::ppu::Tile;

/// Slices art whose sides are multiples of 8 into tiles, row-major. A 16×16
/// image yields top-left, top-right, bottom-left, bottom-right, which is the
/// GBA's 1D sprite mapping.
pub fn tiles<S: AsRef<str>>(art: &[S]) -> Vec<Tile> {
    tiles_offset(art, 0)
}

/// Like `tiles`, but shifts every index up by one so `.` becomes color 1:
/// for fully opaque background art drawn with the 4-color placeholder set.
pub fn opaque_tiles<S: AsRef<str>>(art: &[S]) -> Vec<Tile> {
    tiles_offset(art, 1)
}

fn tiles_offset<S: AsRef<str>>(art: &[S], offset: u8) -> Vec<Tile> {
    let h = art.len();
    let w = art[0].as_ref().len();
    assert!(w % 8 == 0 && h % 8 == 0, "art is {w}×{h}, sides must be multiples of 8");
    for (i, row) in art.iter().enumerate() {
        assert_eq!(row.as_ref().len(), w, "art row {i} has the wrong width: {:?}", row.as_ref());
    }

    let mut out = Vec::with_capacity(w * h / 64);
    for ty in (0..h).step_by(8) {
        for tx in (0..w).step_by(8) {
            let mut t = [0u8; 32];
            for y in 0..8 {
                let row = art[ty + y].as_ref().as_bytes();
                for x in 0..8 {
                    let c = index(row[tx + x]) + offset;
                    assert!(c < 16, "color index {c} out of range");
                    t[y * 4 + x / 2] |= c << (4 * (x % 2));
                }
            }
            out.push(t);
        }
    }
    out
}

fn index(c: u8) -> u8 {
    match c {
        b'.' => 0,
        b'+' => 1,
        b'x' => 2,
        b'#' => 3,
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("unknown art pixel {:?}", c as char),
    }
}

/// Rotates every row left by `dx` pixels; used to derive animation frames.
pub fn shifted(art: &[&str], dx: usize) -> Vec<String> {
    art.iter()
        .map(|row| {
            let dx = dx % row.len();
            format!("{}{}", &row[dx..], &row[..dx])
        })
        .collect()
}

/// A 5×7 glyph given as rows separated by `|`: ink in color 1 with a drop
/// shadow in color 2, on a transparent background.
pub fn glyph(rows: &str) -> Tile {
    let mut ink = [[false; 8]; 8];
    for (y, row) in rows.split('|').enumerate() {
        assert!(y < 7 && row.len() == 5, "malformed glyph {rows:?}");
        for (x, c) in row.bytes().enumerate() {
            ink[y][x + 1] = c == b'#';
        }
    }
    let mut t = [0u8; 32];
    for y in 0..8 {
        for x in 0..8 {
            let shadow = x > 0 && y > 0 && ink[y - 1][x - 1];
            let c = if ink[y][x] { 1 } else if shadow { 2 } else { 0 };
            t[y * 4 + x / 2] |= c << (4 * (x % 2));
        }
    }
    t
}

/// Tiles in the raw `.4bpp` format written by `assetc`.
#[allow(dead_code)]
pub fn from_4bpp(bytes: &[u8]) -> Vec<Tile> {
    bytes.chunks_exact(32).map(|c| c.try_into().unwrap()).collect()
}

/// A palette bank in the `.pal` format written by `assetc`: 16 little-endian RGB555 words.
#[allow(dead_code)]
pub fn bank_from_bytes(bytes: &[u8]) -> [u16; 16] {
    std::array::from_fn(|i| u16::from_le_bytes([bytes[i * 2], bytes[i * 2 + 1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_left_pixel_in_low_nibble() {
        let t = tiles(&["+x#f....", "........", "........", "........", "........", "........", "........", "........"]);
        assert_eq!(&t[0][..2], &[0x21, 0xF3]);
        assert_eq!(opaque_tiles(&["........"; 8])[0][0], 0x11);
    }
}
