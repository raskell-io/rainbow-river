//! Rainbow River: a Game Boy Advance–style fishing game.
//!
//! The host (web/main.js) calls `rr_frame` 60 times a second with the
//! button state and blits the framebuffer to a canvas. No wasm-bindgen: the
//! whole interface is these few C-ABI exports.

mod art;
mod assets;
mod fishing;
mod game;
mod input;
mod ppu;
mod rng;
mod text;
mod world;

use std::cell::UnsafeCell;

use game::Game;

struct Global(UnsafeCell<Option<Game>>);

// SAFETY: wasm32-unknown-unknown is single-threaded.
unsafe impl Sync for Global {}

static GAME: Global = Global(UnsafeCell::new(None));

#[allow(clippy::mut_from_ref)]
fn game() -> &'static mut Game {
    // SAFETY: single-threaded, and no export holds the reference across calls.
    unsafe { (*GAME.0.get()).as_mut().expect("rr_init must be called first") }
}

#[unsafe(no_mangle)]
pub extern "C" fn rr_init(seed: u32) {
    // SAFETY: as above.
    unsafe { *GAME.0.get() = Some(Game::new(seed)) };
}

/// Advances one frame (1/60 s). `buttons` is a bitmask, see `input::btn`.
#[unsafe(no_mangle)]
pub extern "C" fn rr_frame(buttons: u32) {
    game().frame(buttons as u8);
}

/// 240×160 RGBA8 pixels.
#[unsafe(no_mangle)]
pub extern "C" fn rr_framebuffer() -> *const u8 {
    game().framebuffer().as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn rr_sram() -> *mut u8 {
    game().sram.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn rr_sram_len() -> u32 {
    game::SRAM_SIZE as u32
}

/// Call after writing saved bytes into `rr_sram`.
#[unsafe(no_mangle)]
pub extern "C" fn rr_sram_loaded() {
    game().load_sram();
}

/// Returns 1 (and clears the flag) when SRAM changed since the last call.
#[unsafe(no_mangle)]
pub extern "C" fn rr_sram_take_dirty() -> u32 {
    std::mem::take(&mut game().sram_dirty) as u32
}
