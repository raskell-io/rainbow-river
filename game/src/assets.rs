//! VRAM layout, palettes and the placeholder art.
//!
//! Every graphic here is meant to be replaced with PixelLab art converted by
//! `assetc`: swap an `art::tiles(&…)` call for
//! `art::from_4bpp(include_bytes!(…))` and the bank's colors for
//! `art::bank_from_bytes(include_bytes!(…))`.

use crate::art;
use crate::ppu::{Ppu, Tile, rgb};
use crate::world::Metatile;

/// Background tiles. Font glyphs sit at their ASCII codes (32–95), so text
/// is written to the map byte by byte.
pub mod bg_tile {
    pub const PANEL_CORNER: u16 = 1;
    pub const PANEL_HEDGE: u16 = 2;
    pub const PANEL_VEDGE: u16 = 3;
    pub const PANEL_FILL: u16 = 4;
    pub const ARROW: u16 = 5;
    /// Nine tiles: a meter segment with 0..=8 pixels filled.
    pub const METER: u16 = 8;
    pub const METATILES: u16 = 96;
}

/// Sprite tiles. With 1D mapping each sprite's tiles are consecutive.
pub mod obj_tile {
    /// Six 16×16 frames: down, down-step, up, up-step, side, side-step.
    pub const PLAYER: u16 = 0;
    pub const BOBBER: u16 = 24;
    pub const BOBBER_SINK: u16 = 25;
    pub const BUBBLE: u16 = 26;
    /// 16×16.
    pub const FISH: u16 = 28;
    /// 16×8.
    pub const RIPPLE: u16 = 32;
}

pub mod bank {
    // Background banks 0–6 belong to the metatiles, see `Metatile::palette`.
    pub const PANEL: u8 = 7;
    pub const TEXT: u8 = 8;
    // Sprite banks.
    pub const PLAYER: u8 = 0;
    pub const BOBBER: u8 = 1;
    pub const BUBBLE: u8 = 2;
    pub const FISH: u8 = 3;
    pub const RIPPLE: u8 = 4;
}

const BACKDROP: u16 = rgb(16, 16, 24);

/// Colors from index 1 up; index 0 is transparent on every layer.
const BG_BANKS: [&[u16]; 9] = [
    // 0 grass
    &[rgb(224, 248, 160), rgb(136, 200, 88), rgb(64, 144, 56), rgb(24, 72, 40)],
    // 1 water
    &[rgb(208, 240, 248), rgb(104, 176, 232), rgb(48, 104, 192), rgb(16, 40, 96)],
    // 2 tree
    &[rgb(136, 200, 88), rgb(56, 152, 64), rgb(24, 96, 48), rgb(48, 32, 24)],
    // 3 path
    &[rgb(248, 232, 184), rgb(216, 184, 120), rgb(160, 120, 72), rgb(88, 56, 32)],
    // 4 rock
    &[rgb(136, 200, 88), rgb(208, 208, 200), rgb(128, 128, 136), rgb(48, 48, 64)],
    // 5 flowers
    &[rgb(136, 200, 88), rgb(248, 248, 240), rgb(240, 104, 128), rgb(40, 112, 48)],
    // 6 bridge
    &[rgb(48, 104, 192), rgb(224, 176, 112), rgb(160, 104, 56), rgb(72, 40, 24)],
    // 7 panel: fill, light border, dark border
    &[rgb(248, 244, 228), rgb(168, 176, 200), rgb(40, 48, 72)],
    // 8 text: ink, shadow, accent, meter background
    &[rgb(40, 40, 56), rgb(200, 196, 184), rgb(224, 72, 64), rgb(120, 120, 136)],
];

const OBJ_BANKS: [&[u16]; 5] = [
    // 0 player
    &[rgb(248, 208, 168), rgb(224, 64, 56), rgb(32, 32, 48)],
    // 1 bobber
    &[rgb(248, 248, 248), rgb(232, 56, 48), rgb(32, 32, 48)],
    // 2 bubble
    &[rgb(248, 248, 248), rgb(248, 200, 64), rgb(32, 32, 48)],
    // 3 fish
    &[rgb(200, 224, 240), rgb(96, 136, 184), rgb(32, 48, 80)],
    // 4 ripple
    &[rgb(240, 248, 255)],
];

/// Tile data the game swaps into VRAM over time.
pub struct Animations {
    water: [Vec<Tile>; 2],
}

pub fn load(ppu: &mut Ppu) -> Animations {
    ppu.bg_pal[0] = BACKDROP;
    for (b, colors) in BG_BANKS.iter().enumerate() {
        ppu.bg_pal[b * 16 + 1..][..colors.len()].copy_from_slice(colors);
    }
    for (b, colors) in OBJ_BANKS.iter().enumerate() {
        ppu.obj_pal[b * 16 + 1..][..colors.len()].copy_from_slice(colors);
    }

    let bg = &mut ppu.bg_tiles;
    put(bg, bg_tile::PANEL_CORNER, &art::tiles(&PANEL_CORNER));
    put(bg, bg_tile::PANEL_HEDGE, &art::tiles(&PANEL_HEDGE));
    put(bg, bg_tile::PANEL_VEDGE, &art::tiles(&PANEL_VEDGE));
    put(bg, bg_tile::PANEL_FILL, &art::tiles(&PANEL_FILL));
    put(bg, bg_tile::ARROW, &art::tiles(&ARROW));
    for k in 0..=8 {
        bg[(bg_tile::METER + k) as usize] = meter(k as usize);
    }
    for (ch, rows) in FONT {
        bg[ch as usize] = art::glyph(rows);
    }
    for m in Metatile::ALL {
        put(bg, m.first_tile(), &art::opaque_tiles(metatile_art(m)));
    }

    let obj = &mut ppu.obj_tiles;
    let player: Vec<Tile> = PLAYER.iter().flat_map(|frame| art::tiles(frame)).collect();
    put(obj, obj_tile::PLAYER, &player);
    put(obj, obj_tile::BOBBER, &art::tiles(&BOBBER));
    put(obj, obj_tile::BOBBER_SINK, &art::tiles(&BOBBER_SINK));
    put(obj, obj_tile::BUBBLE, &art::tiles(&BUBBLE));
    put(obj, obj_tile::FISH, &art::tiles(&FISH));
    put(obj, obj_tile::RIPPLE, &art::tiles(&RIPPLE));

    Animations { water: [art::opaque_tiles(&WATER), art::opaque_tiles(&art::shifted(&WATER, 3))] }
}

/// Called every frame: animates water by rewriting its tiles in VRAM.
pub fn animate(ppu: &mut Ppu, anim: &Animations, frame: u32) {
    if frame % 32 == 0 {
        let phase = (frame / 32 % 2) as usize;
        put(&mut ppu.bg_tiles, Metatile::Water.first_tile(), &anim.water[phase]);
    }
}

fn put(vram: &mut [Tile], first: u16, tiles: &[Tile]) {
    let first = first as usize;
    vram[first..first + tiles.len()].copy_from_slice(tiles);
}

fn meter(filled: usize) -> Tile {
    let rows: Vec<String> = (0..8)
        .map(|y| match y {
            1 | 6 => "11111111".to_string(),
            2..=5 => (0..8).map(|x| if x < filled { '3' } else { '4' }).collect(),
            _ => "........".to_string(),
        })
        .collect();
    art::tiles(&rows)[0]
}

fn metatile_art(m: Metatile) -> &'static [&'static str; 16] {
    match m {
        Metatile::Grass => &GRASS,
        Metatile::Flowers => &FLOWERS,
        Metatile::Tree => &TREE,
        Metatile::Water => &WATER,
        Metatile::Path => &PATH,
        Metatile::Rock => &ROCK,
        Metatile::Bridge => &BRIDGE,
    }
}

const PANEL_CORNER: [&str; 8] = [
    "..######",
    ".#xxxxxx",
    "#x++++++",
    "#x++++++",
    "#x++++++",
    "#x++++++",
    "#x++++++",
    "#x++++++",
];

const PANEL_HEDGE: [&str; 8] = [
    "########",
    "xxxxxxxx",
    "++++++++",
    "++++++++",
    "++++++++",
    "++++++++",
    "++++++++",
    "++++++++",
];

const PANEL_VEDGE: [&str; 8] = ["#x++++++"; 8];

const PANEL_FILL: [&str; 8] = ["++++++++"; 8];

const ARROW: [&str; 8] = [
    "........",
    "........",
    ".#####..",
    "..###...",
    "...#....",
    "........",
    "........",
    "........",
];

const RIPPLE: [&str; 8] = [
    "....++++++++....",
    "..++........++..",
    ".+............+.",
    "+..............+",
    ".+............+.",
    "..++........++..",
    "....++++++++....",
    "................",
];

const GRASS: [&str; 16] = [
    "++++++++++++++++",
    "++++++++++++++++",
    "+++x+++++++++.++",
    "++x+x++++++++x++",
    "++++++++++++++++",
    "++++++++++++++++",
    "+++++++++x++++++",
    "++++++++x+x+++++",
    "++++++++++++++++",
    "+.++++++++++++++",
    "+x++++++++++++++",
    "++++++++++++x+++",
    "+++++x+++++x+x++",
    "++++x+x+++++++++",
    "++++++++++++++++",
    "++++++++++++++++",
];

const FLOWERS: [&str; 16] = [
    "................",
    "..+.............",
    ".+x+......+.....",
    "..+......+x+....",
    "..#.......+.....",
    "..#.......#.....",
    "..........#.....",
    "................",
    ".....+..........",
    "....+x+.....+...",
    ".....+.....+x+..",
    ".....#......+...",
    ".....#......#...",
    "............#...",
    "................",
    "................",
];

const TREE: [&str; 16] = [
    ".....######.....",
    "...##++++++##...",
    "..#++++++++xx#..",
    ".#+++.++++++xx#.",
    ".#++..+++++xxx#.",
    "#++++++++++xxxx#",
    "#+++++++++xxxxx#",
    "#x++++++++xxxxx#",
    "#xx+++++xxxxxxx#",
    ".#xxxxxxxxxxxx#.",
    ".#xxxxxxxxxxxx#.",
    "..##xxxxxxxx##..",
    "....########....",
    "......####......",
    "......####......",
    ".....######.....",
];

const WATER: [&str; 16] = [
    "xxxxxxxxxxxxxxxx",
    "xxx+++xxxxxxxxxx",
    "xx+xxx+xxxxxxxxx",
    "xxxxxxxxxxxxxxxx",
    "xxxxxxxxxx++xxxx",
    "xxxxxxxxx+xx+xxx",
    "xxxxxxxxxxxxxxxx",
    "x.xxxxxxxxxxxxxx",
    "xxxxxxxxxxxxxxxx",
    "xxxxx+++xxxxxxxx",
    "xxxx+xxx+xxxxxxx",
    "xxxxxxxxxxxxxx.x",
    "xxxxxxxxxxxxxxxx",
    "xx++xxxxxxxx+++x",
    "x+xx+xxxxxx+xxx+",
    "xxxxxxxxxxxxxxxx",
];

const PATH: [&str; 16] = [
    "++++++++++++++++",
    "++++++++++x+++++",
    "+.++++++++++++++",
    "++++++++++++++++",
    "++++++x+++++++++",
    "++++++++++++.+++",
    "++++++++++++++++",
    "+++x++++++++++++",
    "++++++++++++++++",
    "+++++++++x++++++",
    "++.+++++++++++++",
    "++++++++++++++++",
    "++++++++++++x+++",
    "+++++x++++++++++",
    "++++++++++++++++",
    "++++++++++.+++++",
];

const ROCK: [&str; 16] = [
    "................",
    "................",
    ".....######.....",
    "...##++++++##...",
    "..#++++++++xx#..",
    ".#+++++++++xxx#.",
    ".#++++++++xxxx#.",
    "#++++++++xxxxxx#",
    "#+++++++xxxxxxx#",
    "#x+++++xxxxxxxx#",
    "#xx++xxxxxxxxxx#",
    "#xxxxxxxxxxxxxx#",
    ".#xxxxxxxxxxxx#.",
    "..############..",
    "................",
    "................",
];

const BRIDGE: [&str; 16] = [
    "################",
    "xxxxxxxxxxxxxxxx",
    "################",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "+++#+++++#+++++#",
    "################",
    "xxxxxxxxxxxxxxxx",
    "################",
    "................",
];

/// Down, down-step, up, up-step, side (facing left), side-step. The other
/// walk step and facing right are the same frames flipped horizontally.
const PLAYER: [[&str; 16]; 6] = [
    [
        "................",
        ".....######.....",
        "....#xxxxxx#....",
        "...#xxxxxxxx#...",
        "...##########...",
        "...#++++++++#...",
        "...#+#++++#+#...",
        "...#++++++++#...",
        "....#++++++#....",
        "...#xxxxxxxx#...",
        "..#+#xxxxxx#+#..",
        "..#+#xxxxxx#+#..",
        "....########....",
        "....###..###....",
        "....###..###....",
        "................",
    ],
    [
        ".....######.....",
        "....#xxxxxx#....",
        "...#xxxxxxxx#...",
        "...##########...",
        "...#++++++++#...",
        "...#+#++++#+#...",
        "...#++++++++#...",
        "....#++++++#....",
        "...#xxxxxxxx#...",
        "..#+#xxxxxx#+#..",
        "..#+#xxxxxx#+#..",
        "....########....",
        "....###..###....",
        "....###.........",
        "................",
        "................",
    ],
    [
        "................",
        ".....######.....",
        "....#xxxxxx#....",
        "...#xxxxxxxx#...",
        "...##########...",
        "...##########...",
        "...##########...",
        "...#+######+#...",
        "....#++++++#....",
        "...#xxxxxxxx#...",
        "..#+#xxxxxx#+#..",
        "..#+#xxxxxx#+#..",
        "....########....",
        "....###..###....",
        "....###..###....",
        "................",
    ],
    [
        ".....######.....",
        "....#xxxxxx#....",
        "...#xxxxxxxx#...",
        "...##########...",
        "...##########...",
        "...##########...",
        "...#+######+#...",
        "....#++++++#....",
        "...#xxxxxxxx#...",
        "..#+#xxxxxx#+#..",
        "..#+#xxxxxx#+#..",
        "....########....",
        "....###..###....",
        "....###.........",
        "................",
        "................",
    ],
    [
        "................",
        "......######....",
        ".....#xxxxxx#...",
        "....#xxxxxxxx#..",
        "..#######xxxx#..",
        "...#+++++####...",
        "...#+#+++++##...",
        "...#+++++++##...",
        "....#+++++##....",
        "....#xxxxxx#....",
        "....#x+#xxx#....",
        "....#x+#xxx#....",
        "....########....",
        ".....##..##.....",
        ".....##..##.....",
        "................",
    ],
    [
        ".......######...",
        "......#xxxxxx#..",
        ".....#xxxxxxxx#.",
        "...#######xxxx#.",
        "....#+++++####..",
        "....#+#+++++##..",
        "....#+++++++##..",
        ".....#+++++##...",
        ".....#xxxxxx#...",
        ".....#xx+#xx#...",
        ".....#xx+#xx#...",
        ".....########...",
        "....##.....##...",
        "...##.......##..",
        "................",
        "................",
    ],
];

const BOBBER: [&str; 8] = [
    "........",
    "...##...",
    "..#xx#..",
    "..#xx#..",
    "..#++#..",
    "..#++#..",
    "...##...",
    "........",
];

const BOBBER_SINK: [&str; 8] = [
    "........",
    "........",
    "........",
    "...##...",
    "..#xx#..",
    ".+#xx#+.",
    "+......+",
    "........",
];

const BUBBLE: [&str; 8] = [
    ".######.",
    "#++##++#",
    "#++##++#",
    "#++##++#",
    "#++++++#",
    "#++##++#",
    ".######.",
    "...#....",
];

const FISH: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    ".....######.....",
    "...##++++++##.##",
    "..#+#+++++++##x#",
    ".#++++++++++#xx#",
    ".#xxxxxxxxxxxxx#",
    "..#xxxxxxxxx##x#",
    "...##xxxxxx##.##",
    ".....######.....",
    "................",
    "................",
    "................",
    "................",
];

/// 5×7 glyphs. Uppercase only, as on many Game Boy games.
const FONT: [(u8, &str); 48] = [
    (b' ', ".....|.....|.....|.....|.....|.....|....."),
    (b'A', ".###.|#...#|#...#|#####|#...#|#...#|#...#"),
    (b'B', "####.|#...#|#...#|####.|#...#|#...#|####."),
    (b'C', ".###.|#...#|#....|#....|#....|#...#|.###."),
    (b'D', "####.|#...#|#...#|#...#|#...#|#...#|####."),
    (b'E', "#####|#....|#....|####.|#....|#....|#####"),
    (b'F', "#####|#....|#....|####.|#....|#....|#...."),
    (b'G', ".###.|#...#|#....|#.###|#...#|#...#|.####"),
    (b'H', "#...#|#...#|#...#|#####|#...#|#...#|#...#"),
    (b'I', ".###.|..#..|..#..|..#..|..#..|..#..|.###."),
    (b'J', "..###|...#.|...#.|...#.|...#.|#..#.|.##.."),
    (b'K', "#...#|#..#.|#.#..|##...|#.#..|#..#.|#...#"),
    (b'L', "#....|#....|#....|#....|#....|#....|#####"),
    (b'M', "#...#|##.##|#.#.#|#.#.#|#...#|#...#|#...#"),
    (b'N', "#...#|#...#|##..#|#.#.#|#..##|#...#|#...#"),
    (b'O', ".###.|#...#|#...#|#...#|#...#|#...#|.###."),
    (b'P', "####.|#...#|#...#|####.|#....|#....|#...."),
    (b'Q', ".###.|#...#|#...#|#...#|#.#.#|#..#.|.##.#"),
    (b'R', "####.|#...#|#...#|####.|#.#..|#..#.|#...#"),
    (b'S', ".###.|#...#|#....|.###.|....#|#...#|.###."),
    (b'T', "#####|..#..|..#..|..#..|..#..|..#..|..#.."),
    (b'U', "#...#|#...#|#...#|#...#|#...#|#...#|.###."),
    (b'V', "#...#|#...#|#...#|#...#|#...#|.#.#.|..#.."),
    (b'W', "#...#|#...#|#...#|#.#.#|#.#.#|#.#.#|.#.#."),
    (b'X', "#...#|#...#|.#.#.|..#..|.#.#.|#...#|#...#"),
    (b'Y', "#...#|#...#|.#.#.|..#..|..#..|..#..|..#.."),
    (b'Z', "#####|....#|...#.|..#..|.#...|#....|#####"),
    (b'0', ".###.|#...#|#..##|#.#.#|##..#|#...#|.###."),
    (b'1', "..#..|.##..|..#..|..#..|..#..|..#..|.###."),
    (b'2', ".###.|#...#|....#|...#.|..#..|.#...|#####"),
    (b'3', "####.|....#|....#|.###.|....#|....#|####."),
    (b'4', "...#.|..##.|.#.#.|#..#.|#####|...#.|...#."),
    (b'5', "#####|#....|####.|....#|....#|#...#|.###."),
    (b'6', ".###.|#....|#....|####.|#...#|#...#|.###."),
    (b'7', "#####|....#|...#.|..#..|.#...|.#...|.#..."),
    (b'8', ".###.|#...#|#...#|.###.|#...#|#...#|.###."),
    (b'9', ".###.|#...#|#...#|.####|....#|....#|.###."),
    (b'!', "..#..|..#..|..#..|..#..|..#..|.....|..#.."),
    (b'?', ".###.|#...#|....#|...#.|..#..|.....|..#.."),
    (b'.', ".....|.....|.....|.....|.....|.....|..#.."),
    (b',', ".....|.....|.....|.....|.....|..#..|.#..."),
    (b'\'', "..#..|..#..|.#...|.....|.....|.....|....."),
    (b'-', ".....|.....|.....|.###.|.....|.....|....."),
    (b':', ".....|..#..|.....|.....|.....|..#..|....."),
    (b'/', "....#|....#|...#.|..#..|.#...|#....|#...."),
    (b'>', ".#...|..#..|...#.|....#|...#.|..#..|.#..."),
    (b'(', "...#.|..#..|.#...|.#...|.#...|..#..|...#."),
    (b')', ".#...|..#..|...#.|...#.|...#.|..#..|.#..."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_assets_load() {
        let mut ppu = Ppu::new();
        load(&mut ppu);
    }
}
