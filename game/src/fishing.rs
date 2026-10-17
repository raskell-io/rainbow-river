//! Cast, wait for the bite, strike, then reel while managing line tension.

use crate::assets::obj_tile as tile;
use crate::input::{Input, btn};
use crate::rng::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Waters {
    River,
    Pond,
}

pub struct Species {
    pub name: &'static str,
    pub waters: Waters,
    pub min_cm: u16,
    pub max_cm: u16,
    /// Relative odds of biting within its waters.
    pub weight: u32,
    /// 1–5: how hard it fights.
    pub strength: u8,
}

pub const SPECIES: [Species; 9] = [
    Species { name: "DACE", waters: Waters::River, min_cm: 10, max_cm: 20, weight: 40, strength: 1 },
    Species { name: "CHUB", waters: Waters::River, min_cm: 15, max_cm: 40, weight: 30, strength: 2 },
    Species { name: "RAINBOW TROUT", waters: Waters::River, min_cm: 25, max_cm: 60, weight: 18, strength: 3 },
    Species { name: "CATFISH", waters: Waters::River, min_cm: 40, max_cm: 95, weight: 8, strength: 4 },
    Species { name: "GOLDEN SALMON", waters: Waters::River, min_cm: 60, max_cm: 110, weight: 2, strength: 5 },
    Species { name: "BLUEGILL", waters: Waters::Pond, min_cm: 10, max_cm: 22, weight: 40, strength: 1 },
    Species { name: "CRUCIAN CARP", waters: Waters::Pond, min_cm: 12, max_cm: 30, weight: 35, strength: 2 },
    Species { name: "CARP", waters: Waters::Pond, min_cm: 30, max_cm: 85, weight: 18, strength: 3 },
    Species { name: "SNAKEHEAD", waters: Waters::Pond, min_cm: 40, max_cm: 90, weight: 7, strength: 4 },
];

fn pick(rng: &mut Rng, waters: Waters) -> usize {
    let candidates = || SPECIES.iter().enumerate().filter(|(_, s)| s.waters == waters);
    let total: u32 = candidates().map(|(_, s)| s.weight).sum();
    let mut roll = rng.range(0, total - 1);
    for (i, s) in candidates() {
        if roll < s.weight {
            return i;
        }
        roll -= s.weight;
    }
    unreachable!("weights cover the roll")
}

const CAST_FRAMES: u16 = 24;
const TENSION_MAX: f32 = 100.0;

enum Phase {
    Cast { t: u16 },
    Wait { timer: u16, nibble: u16 },
    Bite { window: u16, species: usize, cm: u16 },
    Reel(Reel),
}

struct Reel {
    species: usize,
    cm: u16,
    start: f32,
    distance: f32,
    tension: f32,
    pulling: bool,
    timer: u16,
}

pub enum Outcome {
    Continue,
    Cancelled,
    Caught { species: usize, cm: u16 },
    Lost(&'static str),
}

pub struct Fishing {
    phase: Phase,
    waters: Waters,
    /// World pixels (sprite top-left) where the cast starts and lands.
    origin: (i32, i32),
    target: (i32, i32),
    frame: u32,
}

impl Fishing {
    pub fn new(waters: Waters, origin: (i32, i32), target: (i32, i32)) -> Self {
        Self { phase: Phase::Cast { t: 0 }, waters, origin, target, frame: 0 }
    }

    pub fn update(&mut self, input: &Input, rng: &mut Rng) -> Outcome {
        self.frame += 1;
        match &mut self.phase {
            Phase::Cast { t } => {
                *t += 1;
                if *t >= CAST_FRAMES {
                    self.phase = Phase::Wait { timer: rng.range(60, 300) as u16, nibble: 0 };
                }
            }
            Phase::Wait { timer, nibble } => {
                if input.pressed(btn::A) || input.pressed(btn::B) {
                    return Outcome::Cancelled;
                }
                // Nibbles are pure feints: the bobber dips, nothing to strike.
                if *nibble > 0 {
                    *nibble -= 1;
                } else if rng.chance(1, 90) {
                    *nibble = 8;
                }
                *timer -= 1;
                if *timer == 0 {
                    let species = pick(rng, self.waters);
                    let s = &SPECIES[species];
                    let cm = rng.range(s.min_cm as u32, s.max_cm as u32) as u16;
                    let window = 26 - 2 * s.strength as u16;
                    self.phase = Phase::Bite { window, species, cm };
                }
            }
            Phase::Bite { window, species, cm } => {
                if input.pressed(btn::A) {
                    let start = 30.0 + 6.0 * SPECIES[*species].strength as f32;
                    self.phase = Phase::Reel(Reel {
                        species: *species,
                        cm: *cm,
                        start,
                        distance: start,
                        tension: 0.0,
                        pulling: true,
                        timer: 40,
                    });
                } else {
                    *window -= 1;
                    if *window == 0 {
                        return Outcome::Lost("IT GOT AWAY...");
                    }
                }
            }
            Phase::Reel(r) => return r.update(input, rng),
        }
        Outcome::Continue
    }

    /// The bobber's sprite: world position and tile.
    pub fn bobber(&self) -> (i32, i32, u16) {
        let (ox, oy) = self.origin;
        let (tx, ty) = self.target;
        let lerp = |a: i32, b: i32, f: f32| a + ((b - a) as f32 * f) as i32;
        match &self.phase {
            Phase::Cast { t } => {
                let f = *t as f32 / CAST_FRAMES as f32;
                let arc = (f * std::f32::consts::PI).sin() * 20.0;
                (lerp(ox, tx, f), lerp(oy, ty, f) - arc as i32, tile::BOBBER)
            }
            Phase::Wait { nibble, .. } => {
                let bob = (self.frame / 24 % 2) as i32;
                (tx, ty + bob + if *nibble > 0 { 2 } else { 0 }, tile::BOBBER)
            }
            Phase::Bite { .. } => (tx, ty, tile::BOBBER_SINK),
            Phase::Reel(r) => {
                let f = 1.0 - r.distance / r.start;
                let jitter = if r.pulling { (self.frame / 2 % 3) as i32 - 1 } else { 0 };
                (lerp(tx, ox, f) + jitter, lerp(ty, oy, f), tile::BOBBER_SINK)
            }
        }
    }

    /// Where to draw a ripple around the bobber while a fish is biting.
    pub fn ripple(&self) -> Option<(i32, i32)> {
        matches!(self.phase, Phase::Bite { .. }).then_some((self.target.0 - 4, self.target.1 + 3))
    }

    /// Whether to show the "!" above the player.
    pub fn alert(&self) -> bool {
        matches!(self.phase, Phase::Bite { .. })
    }

    /// While reeling: tension in 0..=1 and remaining line in meters.
    pub fn hud(&self) -> Option<(f32, u16)> {
        match &self.phase {
            Phase::Reel(r) => Some((r.tension / TENSION_MAX, r.distance.ceil() as u16)),
            _ => None,
        }
    }
}

impl Reel {
    fn update(&mut self, input: &Input, rng: &mut Rng) -> Outcome {
        let strength = SPECIES[self.species].strength as f32;

        // The fish alternates between pulling hard and resting.
        self.timer = self.timer.saturating_sub(1);
        if self.timer == 0 {
            self.pulling = !self.pulling;
            self.timer = if self.pulling {
                rng.range(30, 70) as u16
            } else {
                rng.range(40, 130 - 15 * strength as u32) as u16
            };
        }

        if input.held(btn::A) {
            self.distance -= 0.35;
            self.tension += if self.pulling { 0.9 + 0.35 * strength } else { 0.6 };
        } else {
            self.tension -= 1.2;
            if self.pulling {
                self.distance += 0.1 + 0.05 * strength;
            }
        }
        self.tension = self.tension.clamp(0.0, TENSION_MAX);

        if self.tension >= TENSION_MAX {
            Outcome::Lost("THE LINE SNAPPED!")
        } else if self.distance > self.start + 15.0 {
            Outcome::Lost("IT SWAM AWAY...")
        } else if self.distance <= 0.0 {
            Outcome::Caught { species: self.species, cm: self.cm }
        } else {
            Outcome::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_only_from_the_right_waters() {
        let mut rng = Rng::new(7);
        for _ in 0..500 {
            assert_eq!(SPECIES[pick(&mut rng, Waters::Pond)].waters, Waters::Pond);
            assert_eq!(SPECIES[pick(&mut rng, Waters::River)].waters, Waters::River);
        }
    }

    #[test]
    fn names_fit_a_text_line() {
        for s in &SPECIES {
            assert!(s.name.len() <= crate::text::LINE_CHARS - 1, "{}", s.name);
        }
    }

    #[test]
    fn reeling_through_calm_water_lands_a_weak_fish() {
        let mut input = Input::default();
        input.update(btn::A);
        let mut rng = Rng::new(1);
        let mut reel = Reel {
            species: 0,
            cm: 12,
            start: 36.0,
            distance: 36.0,
            tension: 0.0,
            pulling: false,
            timer: 1000,
        };
        let outcome = loop {
            match reel.update(&input, &mut rng) {
                Outcome::Continue => {}
                other => break other,
            }
        };
        assert!(matches!(outcome, Outcome::Caught { species: 0, cm: 12 }));
    }
}
