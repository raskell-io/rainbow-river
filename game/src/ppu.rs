//! A Game Boy Advance–style picture processor (tile mode 0).
//!
//! The game never draws pixels directly. It uploads 8×8 4bpp tiles, arranges
//! them on up to four background layers, places sprites in OAM, picks
//! palettes and configures blending. The PPU composes each frame from that
//! state, scanline by scanline, following the GBA's rules: 16 palette banks
//! of 16 colors per layer type, color 0 transparent, layer priorities 0–3,
//! sprites from 8×8 to 64×64 with optional rotation/scaling, alpha blending
//! and brightness fades.

pub const SCREEN_W: usize = 240;
pub const SCREEN_H: usize = 160;
pub const BG_TILES: usize = 2048;
pub const OBJ_TILES: usize = 1024;
pub const BG_COUNT: usize = 4;
pub const OAM_SIZE: usize = 128;
pub const AFFINE_SLOTS: usize = 32;

/// 8×8 pixels, 4 bits each; the low nibble of each byte is the left pixel.
pub type Tile = [u8; 32];

/// A background map entry, bit-compatible with the GBA's text-mode format:
/// tile (10 bits), h-flip, v-flip, palette bank (4 bits).
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ScreenEntry(pub u16);

impl ScreenEntry {
    pub const EMPTY: Self = Self(0);

    pub const fn new(tile: u16, bank: u8) -> Self {
        Self((tile & 0x3FF) | ((bank as u16 & 0xF) << 12))
    }
    pub const fn hflip(self) -> Self {
        Self(self.0 | 1 << 10)
    }
    pub const fn vflip(self) -> Self {
        Self(self.0 | 1 << 11)
    }

    fn tile(self) -> usize {
        (self.0 & 0x3FF) as usize
    }
    fn bank(self) -> usize {
        (self.0 >> 12) as usize
    }
    fn is_hflip(self) -> bool {
        self.0 & 1 << 10 != 0
    }
    fn is_vflip(self) -> bool {
        self.0 & 1 << 11 != 0
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapSize {
    S32x32,
    S64x32,
    S32x64,
    S64x64,
}

impl MapSize {
    /// Columns and rows, in tiles.
    pub const fn dims(self) -> (usize, usize) {
        match self {
            MapSize::S32x32 => (32, 32),
            MapSize::S64x32 => (64, 32),
            MapSize::S32x64 => (32, 64),
            MapSize::S64x64 => (64, 64),
        }
    }
}

pub struct Background {
    pub enabled: bool,
    /// 0 draws on top, 3 at the back.
    pub priority: u8,
    /// Added to every tile index in the map.
    pub char_base: u16,
    size: MapSize,
    map: Vec<ScreenEntry>,
    /// Scroll offsets; the map wraps around.
    pub hofs: u16,
    pub vofs: u16,
}

impl Background {
    fn new(priority: u8) -> Self {
        let size = MapSize::S32x32;
        let (c, r) = size.dims();
        Self { enabled: false, priority, char_base: 0, size, map: vec![ScreenEntry::EMPTY; c * r], hofs: 0, vofs: 0 }
    }

    #[allow(dead_code)]
    pub fn set_size(&mut self, size: MapSize) {
        let (c, r) = size.dims();
        self.size = size;
        self.map = vec![ScreenEntry::EMPTY; c * r];
    }

    pub fn set(&mut self, col: usize, row: usize, entry: ScreenEntry) {
        let (c, r) = self.size.dims();
        self.map[(row % r) * c + col % c] = entry;
    }

    pub fn clear(&mut self) {
        self.map.fill(ScreenEntry::EMPTY);
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteSize {
    S8x8,
    S16x16,
    S32x32,
    S64x64,
    W16x8,
    W32x8,
    W32x16,
    W64x32,
    T8x16,
    T8x32,
    T16x32,
    T32x64,
}

impl SpriteSize {
    /// Width and height in pixels.
    pub const fn dims(self) -> (i32, i32) {
        match self {
            SpriteSize::S8x8 => (8, 8),
            SpriteSize::S16x16 => (16, 16),
            SpriteSize::S32x32 => (32, 32),
            SpriteSize::S64x64 => (64, 64),
            SpriteSize::W16x8 => (16, 8),
            SpriteSize::W32x8 => (32, 8),
            SpriteSize::W32x16 => (32, 16),
            SpriteSize::W64x32 => (64, 32),
            SpriteSize::T8x16 => (8, 16),
            SpriteSize::T8x32 => (8, 32),
            SpriteSize::T16x32 => (16, 32),
            SpriteSize::T32x64 => (32, 64),
        }
    }
}

/// Rotation/scaling for a sprite. Like the hardware, the matrix maps screen
/// space to texture space (it is the inverse of the visual transform), with
/// 8.8 fixed-point entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffineMatrix {
    pub pa: i16,
    pub pb: i16,
    pub pc: i16,
    pub pd: i16,
}

impl AffineMatrix {
    pub const IDENTITY: Self = Self { pa: 256, pb: 0, pc: 0, pd: 256 };

    /// Rotates by `angle` radians (counter-clockwise) and scales by `sx`, `sy`.
    pub fn rotate_scale(angle: f32, sx: f32, sy: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        let fx = |v: f32| (v * 256.0).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        Self { pa: fx(cos / sx), pb: fx(-sin / sx), pc: fx(sin / sy), pd: fx(cos / sy) }
    }
}

/// One OAM entry. `tile` is the first of the sprite's tiles in OBJ VRAM,
/// which uses 1D mapping: a sprite's tiles are consecutive, row-major.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub visible: bool,
    pub x: i16,
    pub y: i16,
    pub size: SpriteSize,
    pub tile: u16,
    pub bank: u8,
    pub hflip: bool,
    pub vflip: bool,
    pub priority: u8,
    /// Semi-transparent: alpha-blends with the layer below whatever the blend mode.
    pub blend: bool,
    /// Affine slot and double-size flag. Flips are ignored for affine sprites.
    pub affine: Option<(u8, bool)>,
}

impl Sprite {
    pub const HIDDEN: Self = Self {
        visible: false,
        x: 0,
        y: 0,
        size: SpriteSize::S8x8,
        tile: 0,
        bank: 0,
        hflip: false,
        vflip: false,
        priority: 0,
        blend: false,
        affine: None,
    };

    pub const fn new(x: i16, y: i16, size: SpriteSize, tile: u16, bank: u8) -> Self {
        Self { visible: true, x, y, size, tile, bank, ..Self::HIDDEN }
    }
    pub const fn priority(self, priority: u8) -> Self {
        Self { priority, ..self }
    }
    pub const fn hflip_if(self, hflip: bool) -> Self {
        Self { hflip, ..self }
    }
    pub const fn semi_transparent(self) -> Self {
        Self { blend: true, ..self }
    }
    pub const fn affine(self, slot: u8, double_size: bool) -> Self {
        Self { affine: Some((slot, double_size)), ..self }
    }
}

/// Layer bits for blend targets.
#[allow(dead_code)]
pub mod layer {
    pub const BG0: u8 = 1 << 0;
    pub const BG1: u8 = 1 << 1;
    pub const BG2: u8 = 1 << 2;
    pub const BG3: u8 = 1 << 3;
    pub const OBJ: u8 = 1 << 4;
    pub const BACKDROP: u8 = 1 << 5;
    pub const ALL: u8 = 0x3F;
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Off,
    /// top × eva/16 + bottom × evb/16
    Alpha,
    /// Fade `top` layers toward white by evy/16.
    Brighten,
    /// Fade `top` layers toward black by evy/16.
    Darken,
}

/// The GBA's BLDCNT/BLDALPHA/BLDY registers. Coefficients run 0..=16.
#[derive(Clone, Copy, Debug)]
pub struct Blend {
    pub mode: BlendMode,
    pub top: u8,
    pub bottom: u8,
    pub eva: u8,
    pub evb: u8,
    pub evy: u8,
}

impl Blend {
    pub const OFF: Self = Self { mode: BlendMode::Off, top: 0, bottom: 0, eva: 16, evb: 0, evy: 0 };
}

pub struct Ppu {
    pub bg_tiles: Vec<Tile>,
    pub obj_tiles: Vec<Tile>,
    /// 16 banks × 16 colors, RGB555. `bg_pal[0]` is the backdrop color.
    pub bg_pal: [u16; 256],
    pub obj_pal: [u16; 256],
    pub bg: [Background; BG_COUNT],
    pub oam: [Sprite; OAM_SIZE],
    pub affine: [AffineMatrix; AFFINE_SLOTS],
    pub blend: Blend,
    framebuffer: Vec<u32>,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
struct ObjPixel {
    color: u16,
    priority: u8,
    blend: bool,
    opaque: bool,
}

impl ObjPixel {
    const NONE: Self = Self { color: 0, priority: 4, blend: false, opaque: false };
}

#[derive(Clone, Copy)]
struct Hit {
    layer: u8,
    color: u16,
    priority: u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            bg_tiles: vec![[0; 32]; BG_TILES],
            obj_tiles: vec![[0; 32]; OBJ_TILES],
            bg_pal: [0; 256],
            obj_pal: [0; 256],
            bg: std::array::from_fn(|i| Background::new(i as u8)),
            oam: [Sprite::HIDDEN; OAM_SIZE],
            affine: [AffineMatrix::IDENTITY; AFFINE_SLOTS],
            blend: Blend::OFF,
            framebuffer: vec![0; SCREEN_W * SCREEN_H],
        }
    }

    /// RGBA8 pixels, row-major, ready for a canvas `ImageData`.
    pub fn framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }

    pub fn render(&mut self) {
        let mut objs = [ObjPixel::NONE; SCREEN_W];
        for ly in 0..SCREEN_H {
            self.sprite_line(ly as i32, &mut objs);
            for lx in 0..SCREEN_W {
                // Find the top two layers at this pixel. Candidates are visited
                // in tie-break order (sprites, then BG0..BG3), so a strict `<`
                // keeps the earlier one when priorities are equal.
                let backdrop = Hit { layer: layer::BACKDROP, color: self.bg_pal[0], priority: 4 };
                let (mut first, mut second) = (backdrop, backdrop);
                let mut consider = |hit: Hit| {
                    if hit.priority < first.priority {
                        second = first;
                        first = hit;
                    } else if hit.priority < second.priority {
                        second = hit;
                    }
                };
                let obj = objs[lx];
                if obj.opaque {
                    consider(Hit { layer: layer::OBJ, color: obj.color, priority: obj.priority });
                }
                for i in 0..BG_COUNT {
                    if let Some(color) = self.bg_pixel(i, lx, ly) {
                        consider(Hit { layer: 1 << i, color, priority: self.bg[i].priority });
                    }
                }

                let b = self.blend;
                let alpha = |a: Hit, c: Hit| mix(a.color, c.color, b.eva, b.evb);
                let color = if first.layer == layer::OBJ && obj.blend && b.bottom & second.layer != 0 {
                    alpha(first, second)
                } else if b.top & first.layer != 0 {
                    match b.mode {
                        BlendMode::Alpha if b.bottom & second.layer != 0 => alpha(first, second),
                        BlendMode::Brighten => mix(first.color, 0x7FFF, 16 - b.evy.min(16), b.evy),
                        BlendMode::Darken => mix(first.color, 0, 16 - b.evy.min(16), b.evy),
                        _ => first.color,
                    }
                } else {
                    first.color
                };
                self.framebuffer[ly * SCREEN_W + lx] = rgba(color);
            }
        }
    }

    fn bg_pixel(&self, i: usize, lx: usize, ly: usize) -> Option<u16> {
        let bg = &self.bg[i];
        if !bg.enabled {
            return None;
        }
        let (cols, rows) = bg.size.dims();
        let px = (lx + bg.hofs as usize) % (cols * 8);
        let py = (ly + bg.vofs as usize) % (rows * 8);
        let entry = bg.map[(py / 8) * cols + px / 8];
        let x = if entry.is_hflip() { 7 - px % 8 } else { px % 8 };
        let y = if entry.is_vflip() { 7 - py % 8 } else { py % 8 };
        let tile = &self.bg_tiles[(bg.char_base as usize + entry.tile()) % BG_TILES];
        match pixel(tile, x, y) {
            0 => None,
            idx => Some(self.bg_pal[entry.bank() * 16 + idx as usize]),
        }
    }

    /// Rasterizes every sprite touching line `ly` into `out`. Among
    /// overlapping opaque pixels the lowest priority number wins, then the
    /// lowest OAM index.
    fn sprite_line(&self, ly: i32, out: &mut [ObjPixel; SCREEN_W]) {
        out.fill(ObjPixel::NONE);
        for s in self.oam.iter().filter(|s| s.visible) {
            let (w, h) = s.size.dims();
            let double = matches!(s.affine, Some((_, true)));
            let (bw, bh) = if double { (w * 2, h * 2) } else { (w, h) };
            let sy = ly - s.y as i32;
            if !(0..bh).contains(&sy) {
                continue;
            }
            let matrix = s.affine.map(|(slot, _)| self.affine[slot as usize % AFFINE_SLOTS]);
            for bx in 0..bw {
                let lx = s.x as i32 + bx;
                if !(0..SCREEN_W as i32).contains(&lx) {
                    continue;
                }
                let dst = &mut out[lx as usize];
                if dst.opaque && dst.priority <= s.priority {
                    continue;
                }
                let (tx, ty) = match matrix {
                    Some(m) => {
                        let (dx, dy) = (bx - bw / 2, sy - bh / 2);
                        let tx = ((m.pa as i32 * dx + m.pb as i32 * dy) >> 8) + w / 2;
                        let ty = ((m.pc as i32 * dx + m.pd as i32 * dy) >> 8) + h / 2;
                        if !(0..w).contains(&tx) || !(0..h).contains(&ty) {
                            continue;
                        }
                        (tx, ty)
                    }
                    None => (if s.hflip { w - 1 - bx } else { bx }, if s.vflip { h - 1 - sy } else { sy }),
                };
                let tile = s.tile as usize + (ty / 8 * (w / 8) + tx / 8) as usize;
                let idx = pixel(&self.obj_tiles[tile % OBJ_TILES], (tx % 8) as usize, (ty % 8) as usize);
                if idx != 0 {
                    *dst = ObjPixel {
                        color: self.obj_pal[s.bank as usize * 16 + idx as usize],
                        priority: s.priority,
                        blend: s.blend,
                        opaque: true,
                    };
                }
            }
        }
    }
}

fn pixel(tile: &Tile, x: usize, y: usize) -> u8 {
    let byte = tile[y * 4 + x / 2];
    if x % 2 == 0 { byte & 0xF } else { byte >> 4 }
}

/// Per-channel `a × ea/16 + b × eb/16`, saturating at 31 like the hardware.
fn mix(a: u16, b: u16, ea: u8, eb: u8) -> u16 {
    let mut out = 0;
    for shift in [0, 5, 10] {
        let ca = (a >> shift) & 31;
        let cb = (b >> shift) & 31;
        let c = ((ca * ea as u16 + cb * eb as u16) / 16).min(31);
        out |= c << shift;
    }
    out
}

/// Packs 8-bit channels into RGB555, the GBA's native color format.
pub const fn rgb(r: u8, g: u8, b: u8) -> u16 {
    (r as u16 >> 3) | ((g as u16 >> 3) << 5) | ((b as u16 >> 3) << 10)
}

fn rgba(c: u16) -> u32 {
    let expand = |v: u16| ((v << 3) | (v >> 2)) as u32;
    let (r, g, b) = (expand(c & 31), expand((c >> 5) & 31), expand((c >> 10) & 31));
    0xFF00_0000 | (b << 16) | (g << 8) | r
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: u16 = rgb(255, 255, 255);
    const RED: u16 = rgb(255, 0, 0);
    const BLUE: u16 = rgb(0, 0, 255);

    fn solid(index: u8) -> Tile {
        [index | index << 4; 32]
    }

    fn px(ppu: &Ppu, x: usize, y: usize) -> u32 {
        ppu.framebuffer()[y * SCREEN_W + x]
    }

    /// BG2 filled with color 1 (blue), backdrop white, sprite bank 0 color 1 red.
    fn scene() -> Ppu {
        let mut ppu = Ppu::new();
        ppu.bg_pal[0] = WHITE;
        ppu.bg_pal[1] = BLUE;
        ppu.obj_pal[1] = RED;
        ppu.bg_tiles[1] = solid(1);
        ppu.obj_tiles[0] = solid(1);
        ppu.bg[2].enabled = true;
        ppu.bg[2].priority = 2;
        for row in 0..32 {
            for col in 0..32 {
                ppu.bg[2].set(col, row, ScreenEntry::new(1, 0));
            }
        }
        ppu
    }

    #[test]
    fn decodes_4bpp_nibbles() {
        let mut t = [0u8; 32];
        t[0] = 0x21; // pixel 0 = 1, pixel 1 = 2
        t[5] = 0xF0; // row 1, pixel 3 = 15
        assert_eq!(pixel(&t, 0, 0), 1);
        assert_eq!(pixel(&t, 1, 0), 2);
        assert_eq!(pixel(&t, 3, 1), 15);
    }

    #[test]
    fn color_zero_shows_the_backdrop() {
        let mut ppu = scene();
        ppu.bg[2].set(0, 0, ScreenEntry::EMPTY);
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(WHITE));
        assert_eq!(px(&ppu, 8, 0), rgba(BLUE));
    }

    #[test]
    fn sprite_priority_against_background() {
        let mut ppu = scene();
        ppu.oam[0] = Sprite::new(0, 0, SpriteSize::S8x8, 0, 0).priority(2);
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(RED), "sprites win ties with BGs");
        ppu.oam[0].priority = 3;
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(BLUE));
    }

    #[test]
    fn large_sprites_use_1d_tile_mapping() {
        let mut ppu = scene();
        ppu.obj_pal[2] = WHITE;
        ppu.obj_tiles[11] = solid(2); // second tile of a 16×16 starting at 10
        ppu.obj_tiles[10] = solid(1);
        ppu.oam[0] = Sprite::new(0, 0, SpriteSize::S16x16, 10, 0);
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(RED));
        assert_eq!(px(&ppu, 8, 0), rgba(WHITE));
    }

    #[test]
    fn alpha_blends_top_over_bottom() {
        let mut ppu = scene();
        ppu.oam[0] = Sprite::new(0, 0, SpriteSize::S8x8, 0, 0).semi_transparent();
        ppu.blend = Blend { mode: BlendMode::Off, top: 0, bottom: layer::BG2, eva: 8, evb: 8, evy: 0 };
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(mix(RED, BLUE, 8, 8)));
        assert_eq!(mix(RED, BLUE, 8, 8), 15 | 15 << 10, "31 × 8/16 rounds down");
    }

    #[test]
    fn darken_fades_to_black() {
        let mut ppu = scene();
        ppu.blend = Blend { mode: BlendMode::Darken, top: layer::ALL, bottom: 0, eva: 0, evb: 0, evy: 16 };
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(0));
    }

    #[test]
    fn affine_identity_matches_plain_sprite_and_scale_halves() {
        let mut ppu = scene();
        ppu.oam[0] = Sprite::new(0, 0, SpriteSize::S16x16, 0, 0).affine(0, false);
        for t in 0..4 {
            ppu.obj_tiles[t] = solid(1);
        }
        ppu.render();
        assert_eq!(px(&ppu, 0, 0), rgba(RED));
        assert_eq!(px(&ppu, 15, 15), rgba(RED));

        // Scaled to 50% the sprite covers only the middle 8×8.
        ppu.affine[0] = AffineMatrix::rotate_scale(0.0, 0.5, 0.5);
        ppu.render();
        assert_eq!(px(&ppu, 2, 2), rgba(BLUE));
        assert_eq!(px(&ppu, 8, 8), rgba(RED));
    }
}
