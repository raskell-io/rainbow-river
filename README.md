# Rainbow River

A Game Boy Advance–style fishing game in Rust + WebAssembly, inspired by
*The Legend of the River King*.

```sh
make serve     # builds web/rainbow-river.wasm, serves on http://localhost:8642 (PORT=… to change)
make test
```

Controls: arrows/WASD move · X/K = A · Z/J = B (hold to run) · Enter = Start (fish log).
Gamepads work too. The fish log is saved to `localStorage` like battery-backed SRAM.

## How it's built

The game never touches pixels. It drives a virtual GBA picture processor
(`game/src/ppu.rs`, tile mode 0) the way a cartridge drives the hardware:

| | Rainbow River | Game Boy Advance |
|---|---|---|
| Screen | 240×160 | 240×160 |
| Tiles | 8×8, 4bpp; 2048 BG + 1024 OBJ | same (8bpp mode not implemented) |
| Palettes | 16 BG + 16 OBJ banks × 16 colors, RGB555, color 0 transparent | same |
| Backgrounds | 4 text layers, priority 0–3, scroll, 32/64-tile maps | same (affine BGs not implemented) |
| Sprites | 128, all 12 sizes 8×8…64×64, 1D tile mapping, priority, flips | same |
| Rotation/scaling | 32 affine matrices, double-size flag | same |
| Blending | alpha (EVA/EVB), brighten/darken (EVY), semi-transparent sprites | same |
| Not yet | mosaic, windows (WIN0/WIN1/OBJ window), per-scanline effects | |

Layer use in the game: BG0 text, BG1 the text panel (alpha-blended over
the world), BG2 the world, sprites between them. The boot fade uses the
darken unit; a fish bite shows a semi-transparent ripple sprite; a catch
pops in with an affine sprite.

The wasm interface is a handful of C-ABI exports (`rr_init`, `rr_frame`,
`rr_framebuffer`, `rr_sram*`); `web/main.js` is the whole host: fixed
60 Hz timestep, input bitmask in, RGBA framebuffer out. No wasm-bindgen.

```
game/src/
  ppu.rs      virtual GBA video hardware
  assets.rs   VRAM layout, palette banks, placeholder art, font
  art.rs      ASCII art / converted data → 4bpp tiles
  world.rs    overworld map and metatiles
  fishing.rs  species, cast → wait → bite → reel state machine
  text.rs     text box (BG0 text over BG1 panel)
  game.rs     player, camera, modes, journal, save RAM
tools/assetc  PNG → .4bpp/.pal converter
web/          index.html + main.js host
```

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

- Sound: GBA-style audio (the 4 legacy channels + 2 PCM) through Web Audio
- Shore/edge metatiles, more areas, NPCs, lures and rods, day/night palettes
- Mosaic and windows; affine backgrounds (mode 1/2) for a map or splash screen
- Touch controls for phones
