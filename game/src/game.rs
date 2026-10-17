use crate::assets::{self, Animations, bank, obj_tile};
use crate::fishing::{Fishing, Outcome, SPECIES};
use crate::input::{Input, btn};
use crate::rng::Rng;
use crate::text::{self, TextBox};
use crate::world::{self, World};
use wasm_ad::addr::{SCREEN_H, SCREEN_W};
use wasm_ad::{AffineMatrix, BgCnt, Sprite, SpriteSize, blend, dispcnt, layer, oam, reg, screen_block};

const SAVE_MAGIC: &[u8; 4] = b"RRVR";
const SAVE_VERSION: u8 = 1;

const FADE_FRAMES: u16 = 32;
/// Layer roles: BG0 text, BG1 text panel, BG2 world.
const WORLD_BG: usize = 2;
const WORLD_BLOCK: u16 = 30;
/// Sprites share the world's priority so they draw above it but below the panel.
const SPRITE_PRIORITY: u16 = 2;
const BASE_DISPCNT: u16 = dispcnt::MODE0 | dispcnt::BG2 | dispcnt::OBJ | dispcnt::OBJ_1D;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Down,
    Up,
    Left,
    Right,
}

impl Dir {
    fn delta(self) -> (i32, i32) {
        match self {
            Dir::Down => (0, 1),
            Dir::Up => (0, -1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }
}

struct Player {
    /// Metatile the player occupies (or is walking into).
    tx: i32,
    ty: i32,
    /// World pixel position of the sprite's top-left.
    px: i32,
    py: i32,
    facing: Dir,
    /// Pixels left in the current step.
    step: i32,
    /// Pixels walked without stopping; drives the walk cycle.
    walk: u32,
}

impl Player {
    fn advance(&mut self, speed: i32) {
        let d = speed.min(self.step);
        let (dx, dy) = self.facing.delta();
        self.px += dx * d;
        self.py += dy * d;
        self.step -= d;
        self.walk += d as u32;
    }
}

enum Mode {
    FadeIn(u16),
    Walk,
    Fish(Fishing),
    /// `held` counts frames since a catch, while the fish is held up.
    Message { text: TextBox, held: Option<u32> },
}

#[derive(Clone, Copy, Default)]
struct Record {
    count: u16,
    best_cm: u16,
}

pub struct Game {
    input: Input,
    rng: Rng,
    world: World,
    anim: Animations,
    frame: u32,
    player: Player,
    mode: Mode,
    journal: [Record; SPECIES.len()],
}

impl Game {
    /// Boots the game: uploads graphics, configures layers, restores the save.
    pub fn new(seed: u32) -> Self {
        let anim = assets::load();
        reg::BGCNT[0].write(BgCnt::new().priority(0).screen_block(text::TEXT_BLOCK).0);
        reg::BGCNT[1].write(BgCnt::new().priority(1).screen_block(text::PANEL_BLOCK).0);
        reg::BGCNT[WORLD_BG].write(BgCnt::new().priority(SPRITE_PRIORITY).screen_block(WORLD_BLOCK).0);
        reg::DISPCNT.write(BASE_DISPCNT);

        let (tx, ty) = world::START;
        let mut game = Self {
            input: Input::default(),
            rng: Rng::new(seed),
            world: World::new(),
            anim,
            frame: 0,
            player: Player { tx, ty, px: tx * 16, py: ty * 16, facing: Dir::Down, step: 0, walk: 0 },
            mode: Mode::FadeIn(FADE_FRAMES),
            journal: [Record::default(); SPECIES.len()],
        };
        game.load_save();
        game
    }

    pub fn update(&mut self) {
        self.input.update();
        self.frame = self.frame.wrapping_add(1);
        self.rng.next();

        let mode = std::mem::replace(&mut self.mode, Mode::Walk);
        self.mode = match mode {
            Mode::FadeIn(0) => Mode::Message {
                text: TextBox::new("WELCOME TO RAINBOW RIVER.\nFACE THE WATER AND PRESS A TO CAST."),
                held: None,
            },
            Mode::FadeIn(t) => Mode::FadeIn(t - 1),
            Mode::Walk => self.walk(),
            Mode::Fish(mut fishing) => match fishing.update(&self.input, &mut self.rng) {
                Outcome::Continue => Mode::Fish(fishing),
                Outcome::Cancelled => Mode::Walk,
                Outcome::Lost(msg) => Mode::Message { text: TextBox::new(msg), held: None },
                Outcome::Caught { species, cm } => self.catch(species, cm),
            },
            Mode::Message { mut text, held } => {
                if text.update(&self.input) {
                    Mode::Walk
                } else {
                    Mode::Message { text, held: held.map(|t| t + 1) }
                }
            }
        };

        self.draw();
    }

    fn walk(&mut self) -> Mode {
        let speed = if self.input.held(btn::B) { 2 } else { 1 };
        if self.player.step > 0 {
            self.player.advance(speed);
            return Mode::Walk;
        }

        if self.input.pressed(btn::START) {
            return Mode::Message { text: self.journal_text(), held: None };
        }
        if self.input.pressed(btn::A) {
            if let Some(fishing) = self.cast() {
                return fishing;
            }
        }

        let p = &mut self.player;
        let held = [(btn::UP, Dir::Up), (btn::DOWN, Dir::Down), (btn::LEFT, Dir::Left), (btn::RIGHT, Dir::Right)]
            .into_iter()
            .find(|&(b, _)| self.input.held(b));
        match held {
            Some((_, dir)) => {
                p.facing = dir;
                let (dx, dy) = dir.delta();
                if self.world.walkable(p.tx + dx, p.ty + dy) {
                    p.tx += dx;
                    p.ty += dy;
                    p.step = 16;
                    p.advance(speed);
                }
            }
            None => p.walk = 0,
        }
        Mode::Walk
    }

    /// Casts as far as 3 metatiles out into the water in front of the player.
    fn cast(&self) -> Option<Mode> {
        let p = &self.player;
        let (dx, dy) = p.facing.delta();
        let waters = self.world.waters(p.tx + dx, p.ty + dy)?;
        let mut reach = 1;
        while reach < 3 && self.world.waters(p.tx + dx * (reach + 1), p.ty + dy * (reach + 1)) == Some(waters) {
            reach += 1;
        }
        let target = ((p.tx + dx * reach) * 16 + 4, (p.ty + dy * reach) * 16 + 4);
        Some(Mode::Fish(Fishing::new(waters, (p.px + 4, p.py + 4), target)))
    }

    fn catch(&mut self, species: usize, cm: u16) -> Mode {
        let record = &mut self.journal[species];
        let new_record = cm > record.best_cm && record.count > 0;
        record.count = record.count.saturating_add(1);
        record.best_cm = record.best_cm.max(cm);
        self.save();

        self.player.facing = Dir::Down;
        let mut msg = format!("YOU CAUGHT A {}!\n{cm}CM", SPECIES[species].name);
        if new_record {
            msg.push_str(" - NEW RECORD!");
        }
        Mode::Message { text: TextBox::new(&msg), held: Some(0) }
    }

    fn journal_text(&self) -> TextBox {
        let caught = self.journal.iter().filter(|r| r.count > 0).count();
        let mut pages = vec![["FISH LOG".to_string(), format!("{caught}/{} SPECIES", SPECIES.len())]];
        for (s, r) in SPECIES.iter().zip(&self.journal) {
            if r.count > 0 {
                pages.push([s.name.to_string(), format!("X{}  BEST {}CM", r.count, r.best_cm)]);
            }
        }
        TextBox::from_pages(pages)
    }

    /// Writes the journal to save RAM and asks the host to persist it.
    fn save(&self) {
        let sram = wasm_ad::save_ram();
        sram[..4].copy_from_slice(SAVE_MAGIC);
        sram[4] = SAVE_VERSION;
        for (i, r) in self.journal.iter().enumerate() {
            let at = 5 + i * 4;
            sram[at..at + 2].copy_from_slice(&r.count.to_le_bytes());
            sram[at + 2..at + 4].copy_from_slice(&r.best_cm.to_le_bytes());
        }
        wasm_ad::request_save();
    }

    fn load_save(&mut self) {
        let sram = wasm_ad::save_ram();
        if !wasm_ad::save_restored() || &sram[..4] != SAVE_MAGIC || sram[4] != SAVE_VERSION {
            return;
        }
        for (i, r) in self.journal.iter_mut().enumerate() {
            let at = 5 + i * 4;
            r.count = u16::from_le_bytes([sram[at], sram[at + 1]]);
            r.best_cm = u16::from_le_bytes([sram[at + 2], sram[at + 3]]);
        }
    }

    fn draw(&mut self) {
        assets::animate(&self.anim, self.frame);

        // Camera follows the player, clamped to the map; the visible part of
        // the world is streamed into the wrapping 32×32 map.
        let p = &self.player;
        let cx = (p.px + 8 - SCREEN_W as i32 / 2).clamp(0, self.world.width_px() - SCREEN_W as i32);
        let cy = (p.py + 8 - SCREEN_H as i32 / 2).clamp(0, self.world.height_px() - SCREEN_H as i32);
        reg::BGHOFS[WORLD_BG].write(cx as u16);
        reg::BGVOFS[WORLD_BG].write(cy as u16);
        let map = screen_block(WORLD_BLOCK as usize);
        for ty in cy / 8..=(cy + SCREEN_H as i32 - 1) / 8 {
            for tx in cx / 8..=(cx + SCREEN_W as i32 - 1) / 8 {
                map[(ty as usize % 32) * 32 + tx as usize % 32] = self.world.tile_at(tx, ty);
            }
        }

        // Sprites; lower OAM slots draw on top.
        let (frame, flip) = self.player_frame();
        oam::hide_all();
        let at = |x: i32, y: i32, size: SpriteSize, tile: u16, bank: u16| {
            Sprite::new(x - cx, y - cy, size, tile).bank(bank).priority(SPRITE_PRIORITY)
        };
        match &self.mode {
            Mode::Fish(fishing) => {
                if fishing.alert() {
                    oam::set(0, at(p.px + 4, p.py - 10, SpriteSize::S8x8, obj_tile::BUBBLE, bank::BUBBLE));
                }
                let (bx, by, tile) = fishing.bobber();
                oam::set(1, at(bx, by, SpriteSize::S8x8, tile, bank::BOBBER));
                if let Some((rx, ry)) = fishing.ripple() {
                    oam::set(2, at(rx, ry, SpriteSize::W16x8, obj_tile::RIPPLE, bank::RIPPLE).semi_transparent());
                }
            }
            Mode::Message { held: Some(t), .. } => {
                // The catch pops in with an affine sprite: grow while wobbling.
                let t = *t as f32;
                let grow = (t / 12.0).min(1.0);
                let scale = 0.3 + 0.7 * grow + 0.15 * (t * 0.5).sin() * (1.0 - grow);
                let angle = (t * 0.35).sin() * 0.5 * (1.0 - (t / 40.0).min(1.0));
                oam::set_affine(0, AffineMatrix::rotate_scale(angle, scale, scale));
                // Double-size: the 16×16 fish renders in a 32×32 box centred on it.
                oam::set(3, at(p.px - 8, p.py - 21, SpriteSize::S16x16, obj_tile::FISH, bank::FISH).affine(0, true));
            }
            _ => {}
        }
        oam::set(8, at(p.px, p.py, SpriteSize::S16x16, obj_tile::PLAYER + frame * 4, bank::PLAYER).hflip(flip));

        match &self.mode {
            Mode::Fish(fishing) => match fishing.hud() {
                Some((tension, meters)) => draw_reel_hud(tension, meters),
                None => text::hide(),
            },
            Mode::Message { text, .. } => text.draw(),
            Mode::Walk | Mode::FadeIn(_) => text::hide(),
        }

        match self.mode {
            Mode::FadeIn(t) => {
                reg::BLDCNT.write(blend::cnt(blend::DARKEN, layer::ALL, 0));
                reg::BLDY.write(t * 16 / FADE_FRAMES);
            }
            // The text panel lets the world and sprites show through; the
            // same coefficients apply to semi-transparent sprites.
            _ => {
                reg::BLDCNT.write(blend::cnt(blend::ALPHA, layer::BG1, layer::BG2 | layer::OBJ));
                reg::BLDALPHA.write(blend::alpha(11, 5));
            }
        }
    }

    /// Which of the six player frames to show, and whether to flip it.
    fn player_frame(&self) -> (u16, bool) {
        let p = &self.player;
        let phase = if p.step > 0 || p.walk > 0 { (p.walk / 8) % 4 } else { 0 };
        let stepping = phase % 2 == 1;
        match p.facing {
            Dir::Down => (stepping as u16, phase == 3),
            Dir::Up => (2 + stepping as u16, phase == 3),
            Dir::Left => (4 + stepping as u16, false),
            Dir::Right => (4 + stepping as u16, true),
        }
    }
}

fn draw_reel_hud(tension: f32, meters: u16) {
    text::show_panel();
    text::put_text(2, 1, "TENSION");
    let filled = (tension * 96.0) as usize;
    for i in 0..12 {
        let segment = filled.saturating_sub(i * 8).min(8) as u16;
        text::put_tile(11 + i, 1, assets::bg_tile::METER + segment);
    }
    text::put_text(2, 3, &format!("LINE {meters:>3}M"));
    text::put_text(20, 3, "HOLD A");
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_ad::host;

    struct Harness {
        game: Game,
        frame: u32,
    }

    impl Harness {
        fn boot(seed: u32) -> Self {
            host::boot(seed);
            Self { game: Game::new(seed), frame: 0 }
        }

        fn run(&mut self, keys: u16, frames: usize) {
            for _ in 0..frames {
                host::frame(self.frame, keys, || self.game.update());
                self.frame += 1;
            }
        }
    }

    #[test]
    fn save_ram_round_trips_the_journal() {
        let mut h = Harness::boot(3);
        h.game.catch(2, 44);
        assert_eq!(wasm_ad::reg::SAVECNT.read() & 1, 1, "catch requests a save");

        // Reboot with the same save RAM, as a host restoring it would.
        let sram = wasm_ad::save_ram().to_vec();
        host::boot(4);
        wasm_ad::save_ram().copy_from_slice(&sram);
        wasm_ad::reg::SAVESTAT.write(1);
        let restored = Game::new(4);
        assert_eq!(restored.journal[2].count, 1);
        assert_eq!(restored.journal[2].best_cm, 44);
    }

    #[test]
    fn walks_and_casts_into_the_river() {
        let mut h = Harness::boot(5);
        // Fade in, let the intro type out, then press A through both pages.
        h.run(0, 200);
        h.run(btn::A, 1);
        h.run(0, 400);
        h.run(btn::A, 1);
        h.run(0, 1);
        assert!(matches!(h.game.mode, Mode::Walk));

        // From the start (7,5), walk right to (11,5), next to the river.
        h.run(btn::RIGHT, 16 * 4);
        h.run(0, 4);
        assert_eq!((h.game.player.tx, h.game.player.ty), (11, 5));
        h.run(btn::A, 1);
        assert!(matches!(h.game.mode, Mode::Fish(_)));
    }

    /// Plays a full catch like a careful player: strike on the bite, then
    /// reel while the tension stays below 70%.
    #[test]
    fn strike_and_reel_lands_a_fish() {
        let mut h = Harness::boot(9);
        h.game.mode = Mode::Walk;
        let p = &mut h.game.player;
        (p.tx, p.ty, p.px, p.py) = (11, 5, 176, 80);
        p.facing = Dir::Right;
        h.run(btn::A, 1);

        let dump = std::env::var("RR_DUMP").ok();
        let mut dumped_bite = false;
        for _ in 0..60 * 60 {
            let keys = match &h.game.mode {
                Mode::Fish(f) if f.alert() => {
                    if let (Some(dir), false) = (&dump, dumped_bite) {
                        write_frame(dir, "bite");
                        dumped_bite = true;
                    }
                    btn::A
                }
                Mode::Fish(f) => match f.hud() {
                    Some((tension, _)) if tension < 0.7 => btn::A,
                    _ => 0,
                },
                _ => break,
            };
            // Edge-triggered strike: release for a frame first.
            if keys == btn::A && !h.game.input.held(btn::A) {
                h.run(0, 1);
            }
            h.run(keys, 1);
            if let Some(dir) = &dump {
                if matches!(&h.game.mode, Mode::Fish(f) if f.hud().is_some()) {
                    write_frame(dir, "reel");
                }
            }
        }
        assert!(matches!(h.game.mode, Mode::Message { held: Some(_), .. }), "expected a catch");
        assert_eq!(h.game.journal.iter().map(|r| r.count).sum::<u16>(), 1);
        if let Some(dir) = &dump {
            h.run(0, 6);
            write_frame(dir, "caught-pop");
            h.run(0, 60);
            write_frame(dir, "caught");
        }
    }

    /// Renders the current frame to a PPM (`RR_DUMP=dir cargo test`).
    fn write_frame(dir: &str, name: &str) {
        host::write_ppm(format!("{dir}/{name}.ppm"), &host::render()).unwrap();
    }
}
