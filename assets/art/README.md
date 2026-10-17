# Art direction

Rainbow River's overworld follows the GBA Pokémon look (Ruby / Sapphire / FireRed).
`mockup.png` is a test screen composed from the assets below.

- Straight-on top-down "3/4" view: front walls and roofs face the camera; everything
  sits on the 16×16 grid. No isometric angles, no ground islands under objects.
- Flat shading, 2–3 tones per surface, crisp dark outlines, at most 15 colours per
  palette bank.
- Buildings around 5×4 tiles (80×64), trees 32×32, player a chibi trainer.

## Sources (generated with PixelLab)

| File | What | How it was made |
|---|---|---|
| `src/terrain-water-wang.png` | grass ↔ water, 16×16 Wang tiles (5×4 sheet) | Tileset tool: square edges, transition 10%, roundness 23%, low detail, basic shading |
| `src/houses.png` | 9 house variants, 80×64 each (4×3 sheet) | Creator Pro, custom 80×64, black outline, style image = the terrain tileset |
| `src/player-idle-8dir.png` | player, 32×32, 8 directions (S, SE, E, NE, N, NW, W, SW) | Character tool v3, low top-down, low detail, black outline |

Wang corner order for the terrain sheet (1 = water), tile (column, row):

```
(0,0) 0001  (1,0) 0011  (2,0) 0010  (3,0) 1110  (4,0) 1101
(0,1) 0101  (1,1) 1111  (2,1) 1010  (3,1) 1011  (4,1) 0111
(0,2) 0100  (1,2) 1100  (2,2) 1000  (3,2) 1001  (4,2) 0110
(0,3) 1111  (1,3) 0000
```
(corners NW NE SW SE)

## Next

- Player: a little smaller (Ruby/Sapphire trainers are ~1.5 tiles tall) and a 3-frame walk.
- Path tileset sharing the grass terrain; tall grass, flowers, fences, ledges, signpost,
  trees that tile in rows, pier and bridge; tackle shop and fisherman's hut; an NPC.
- Then: a build step converting these into 4bpp tiles and palette banks, and the game
  switching from the placeholder art.
