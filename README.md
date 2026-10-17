<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/wordmark-dark.png">
    <img src="assets/brand/wordmark-light.png" alt="Rainbow River" width="420">
  </picture>
</p>

<p align="center"><b>A fishing game inspired by <i>The Legend of the River King</i>.</b></p>

<p align="center">
  <a href="https://wasmad.raskell.io/play/?cart=../carts/rainbow-river.wasm"><b>Play in your browser</b></a> ·
  <a href="https://wasmad.raskell.io">WASM-AD</a>
</p>

Rainbow River is the first cart for [WASM-AD](https://github.com/raskell-io/wasm-ad),
a fantasy console with Game Boy Advance–style hardware whose cartridges are
WebAssembly modules.

```sh
make serve     # builds the cart and the WASM-AD player; http://localhost:8642/?cart=carts/rainbow-river.wasm
make test
```

`make player` expects a wasm-ad checkout next to this one (`WASM_AD=…` to
change). The cart itself is `web/carts/rainbow-river.wasm` and runs in any
WASM-AD player.

Controls: arrows move · X = A · Z = B (hold to run) · Enter = Start (fish log).
The fish log is kept in WASM-AD save RAM.

## How it's built

The game writes WASM-AD's memory-mapped hardware through the `wasm-ad` SDK:
4bpp tiles in character block 0, the world map in screen block 30 streamed as
the camera scrolls, the text box on BG0 (text) over BG1 (an alpha-blended
panel), sprites in OAM. The boot fade uses brightness-decrease, a bite shows
a semi-transparent ripple sprite, and a catch pops in with an affine sprite.

```
game/src/
  lib.rs      cart metadata and the start/update exports
  assets.rs   VRAM layout, palette banks, placeholder art, font
  art.rs      ASCII art / converted data → 4bpp tiles
  world.rs    overworld map and metatiles
  fishing.rs  species, cast → wait → bite → reel state machine
  text.rs     text box (BG0 text over BG1 panel)
  game.rs     player, camera, modes, journal, save RAM
tools/assetc  PNG → .4bpp/.pal converter
```

Tests run natively against the SDK's emulated hardware; `RR_DUMP=dir cargo
test strike` writes frames rendered by the WASM-AD reference renderer.

## Art from PixelLab

All current art is 4-color ASCII placeholders in `assets.rs`; each asset
can have up to 15 colors plus transparency. To replace one:

1. Generate at native size, e.g. 16×16 or 16×32 characters and 16×16
   metatiles, as single images or sheets of equal cells.
2. Convert:
   ```sh
   cargo run -p assetc -- assets/player.png assets/player --cell 16x32
   cargo run -p assetc -- assets/water.png  assets/water --colors 8
   ```
   `--scale N` undoes an N× upscaled export. Check `*.preview.png` for the
   quantized result.
3. In `assets.rs`, swap the `art::tiles(&…)` call for
   `art::from_4bpp(include_bytes!("../../assets/player.4bpp"))` and load
   the bank from `art::bank_from_bytes(include_bytes!(…))`. For a 16×32
   player, change the sprite size in `game.rs` to `SpriteSize::T16x32`.

Colors are ordered light → dark from index 1, matching the placeholders.

## Next

- Sound, once WASM-AD 0.2 specifies audio
- Shore/edge metatiles, more areas, NPCs, lures and rods, day/night palettes
- Raster effects (scanline callback) for shimmering water
