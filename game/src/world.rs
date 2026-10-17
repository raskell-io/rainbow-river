//! The overworld: a grid of 16×16 metatiles, each built from four 8×8 tiles.

use crate::assets::bg_tile;
use crate::fishing::Waters;
use wasm_ad::ScreenEntry;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Metatile {
    Grass,
    Flowers,
    Tree,
    Water,
    Path,
    Rock,
    Bridge,
}

impl Metatile {
    pub const ALL: [Metatile; 7] = [
        Metatile::Grass,
        Metatile::Flowers,
        Metatile::Tree,
        Metatile::Water,
        Metatile::Path,
        Metatile::Rock,
        Metatile::Bridge,
    ];

    pub fn first_tile(self) -> u16 {
        bg_tile::METATILES + self as u16 * 4
    }

    pub fn palette(self) -> u16 {
        match self {
            Metatile::Grass => 0,
            Metatile::Water => 1,
            Metatile::Tree => 2,
            Metatile::Path => 3,
            Metatile::Rock => 4,
            Metatile::Flowers => 5,
            Metatile::Bridge => 6,
        }
    }

    pub fn solid(self) -> bool {
        matches!(self, Metatile::Tree | Metatile::Water | Metatile::Rock)
    }
}

/// `.` grass, `"` flowers, `T` tree, `o` rock, `:` path, `=` bridge,
/// `~` river, `%` pond.
const MAP: &str = r#"
TTTTTTTTTTTT~~~~TTTTTTTTTTTTTT
TT....."...T~~~~T....TT..."..T
T..TT.......~~~~.....T.......T
T..TT..:::::====:::::::......T
T......:....~~~~......:...o..T
T.."...:....~~~~......:......T
T......:.....~~~~.....:..TT..T
TT..o..:.....~~~~.....:..TT..T
T......:......~~~~....:......T
T......:::::::====:::::..."..T
T.TT..........~~~~....:......T
T.TT....."....~~~~....:......T
T.............~~~~....:...%%%T
T....o.......~~~~.....:..%%%%T
T...........~~~~......::%%%%%T
T.."........~~~~.........%%%%T
T..........~~~~~..T.......%%.T
TT........~~~~~...T..........T
T........~~~~~.......o..."...T
T..TT...~~~~~................T
T..TT..~~~~~.......TT........T
T.....~~~~~........TT.....o..T
T....~~~~~...................T
TTTT~~~~~TTTTTTTTTTTTTTTTTTTTT
"#;

/// Where the player starts, in metatiles.
pub const START: (i32, i32) = (7, 5);

pub struct World {
    w: i32,
    h: i32,
    cells: Vec<u8>,
}

impl World {
    pub fn new() -> Self {
        let rows: Vec<&str> = MAP.trim().lines().collect();
        let w = rows[0].len();
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), w, "map row {i} has the wrong width");
        }
        Self { w: w as i32, h: rows.len() as i32, cells: rows.concat().into_bytes() }
    }

    pub fn width_px(&self) -> i32 {
        self.w * 16
    }

    pub fn height_px(&self) -> i32 {
        self.h * 16
    }

    fn cell(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return b'T';
        }
        self.cells[(y * self.w + x) as usize]
    }

    pub fn metatile(&self, x: i32, y: i32) -> Metatile {
        match self.cell(x, y) {
            b'"' => Metatile::Flowers,
            b'T' => Metatile::Tree,
            b'~' | b'%' => Metatile::Water,
            b':' => Metatile::Path,
            b'o' => Metatile::Rock,
            b'=' => Metatile::Bridge,
            _ => Metatile::Grass,
        }
    }

    pub fn waters(&self, x: i32, y: i32) -> Option<Waters> {
        match self.cell(x, y) {
            b'~' => Some(Waters::River),
            b'%' => Some(Waters::Pond),
            _ => None,
        }
    }

    pub fn walkable(&self, x: i32, y: i32) -> bool {
        !self.metatile(x, y).solid()
    }

    /// The background map entry for an 8×8 tile position.
    pub fn tile_at(&self, tx: i32, ty: i32) -> ScreenEntry {
        let m = self.metatile(tx.div_euclid(2), ty.div_euclid(2));
        let quadrant = ty.rem_euclid(2) * 2 + tx.rem_euclid(2);
        ScreenEntry::new(m.first_tile() + quadrant as u16, m.palette())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_is_rectangular_and_start_is_walkable() {
        let world = World::new();
        assert!(world.walkable(START.0, START.1));
    }

    #[test]
    fn bridges_span_the_river() {
        let world = World::new();
        for y in 0..world.h {
            for x in 0..world.w {
                if world.metatile(x, y) == Metatile::Bridge {
                    assert!(world.waters(x, y - 1).is_some() || world.waters(x, y + 1).is_some());
                }
            }
        }
    }
}
