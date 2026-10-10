# Using your own game content

This repository contains **no game assets**. To see the real art, maps and
data, point the engine at your own copy of the game's `Content` folder.

1. Install the game you own (Steam/GOG). Its content lives in
   `<game folder>/Content` as `.xnb` files.
2. Unpack it to plain files with [StardewXNBHack](https://github.com/Pathoschild/StardewXNBHack)
   (run it from the game folder). It writes `Content (unpacked)` containing
   `.png` textures, `.json` data and `.tmx` maps.
3. Run the engine with that folder:

   ```sh
   cargo run --release -p valley_app -- --content "/path/to/Content (unpacked)"
   # or
   VALLEY_CONTENT="/path/to/Content (unpacked)" cargo run --release -p valley_app
   ```

The content folder is layered over the built-in placeholder pack: anything
found in your folder wins, anything missing falls back to placeholders. Raw
`.xnb` files are read directly where possible (uncompressed or LZ4
textures and maps); LZX-compressed files need unpacking first.

Never commit game content to this repository (`.gitignore` blocks the usual
names).

## Asset names

Assets are addressed by their game asset name without extension, e.g.
`Maps/Farm`, `Data/Objects`, `TileSheets/crops`, `Characters/Dialogue/<npc>`.
Lookups are case-insensitive and accept `\` or `/`.

## Compatibility notes

The engine is a clean-room reimplementation; it cannot be checked against the
real files inside this repository. Where it assumes a sprite-sheet layout or a
map convention, the assumption lives in one place so it can be adjusted:

| Area | Where |
|---|---|
| Character sheets (16x32 frames, rows down/right/up/left) | `valley_render::world_draw::character_src` |
| Crop sheet (8 frames of 16x32 per crop, 2 crops per row) | `valley_render::world_draw` (crops) and `valley_sim::crop::Crop::sprite_frame` |
| Tilled soil (4x4 connection variants, watered at +64px) | `valley_render::world_draw` (hoe dirt) |
| Object icons (16x16 grid, columns from texture width) | `valley_render::world_draw::object_sprite` |
| Farmer sprite (currently the placeholder sheet) | `valley_render::world_draw::draw_farmer` |
