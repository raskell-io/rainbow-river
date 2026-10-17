//! Rainbow River: a fishing game for the WASM-AD fantasy console.
//!
//! The cart exports `start` and `update`; everything on screen is produced
//! by writing WASM-AD's memory-mapped video hardware (see wasm-ad/SPEC.md).

mod art;
mod assets;
mod fishing;
mod game;
mod input;
mod rng;
mod text;
mod world;

use std::cell::UnsafeCell;

use game::Game;

wasm_ad::cart! {
    title: "Rainbow River",
    author: "raskell.io",
    version: "0.1.0",
    description: "A fishing game inspired by The Legend of the River King",
    url: "https://github.com/raskell-io/rainbow-river"
}

struct Global(UnsafeCell<Option<Game>>);

// SAFETY: wasm32-unknown-unknown is single-threaded.
unsafe impl Sync for Global {}

static GAME: Global = Global(UnsafeCell::new(None));

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    // SAFETY: single-threaded, and no export holds the reference across calls.
    unsafe { *GAME.0.get() = Some(Game::new(wasm_ad::seed())) };
}

#[unsafe(no_mangle)]
pub extern "C" fn update() {
    // SAFETY: as above.
    if let Some(game) = unsafe { (*GAME.0.get()).as_mut() } {
        game.update();
    }
}
