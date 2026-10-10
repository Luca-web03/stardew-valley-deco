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

## Audio

Sound effects and music come from the game's XACT banks, read at runtime
from your own install (nothing is bundled):

| File | Used for |
|---|---|
| `XACT/Wave Bank.xwb`, `XACT/Wave Bank(1).xwb` (and any other bank the sound bank names) | the audio data |
| `XACT/Sound Bank.xsb` | cue names (e.g. `hoeHit`) -> wave bank entries |

Unpackers such as StardewXNBHack do not copy the `XACT` folder, so besides
the `--content` folder the engine also looks in the packed `Content` folder
next to it (`<game>/Content` when you pass `<game>/Content (unpacked)`), or
in `<dir>/Content` when you pass the game folder itself. You can also copy
`Content/XACT` into your content folder.

Each cue or music track name resolves to the first match of:

1. a loose file `Audio/<name>.wav` or `Audio/<name>.ogg` in the content folder;
2. `Audio/cues.json`, a manual mapping (see below);
3. the waves the cue plays according to `XACT/Sound Bank.xsb` (variations
   are picked at random);
4. a wave bank entry of that name (banks built with entry names);
5. a built-in, procedurally synthesized placeholder (or a soft click for
   unknown cues). Music falls back to generated loops only when no game
   audio is present at all; otherwise an unplayable track is silent.

Supported encodings: wave banks with PCM (8/16-bit) and MS-ADPCM entries;
`.wav` with PCM 8/16/24/32-bit, IEEE float or MS-ADPCM; Ogg Vorbis. xWMA and
XMA wave bank entries cannot be decoded: they are reported once and the
placeholder is used (drop an `Audio/<name>.ogg` in to replace them).

The `.xsb` reader is a clean-room, best-effort implementation of the
publicly documented layout and only extracts cue -> wave mappings (no
categories, volume/pitch variation or RPC curves). If cues are unmapped or
wrong, `Audio/cues.json` overrides them; a plain number is an entry of the
first wave bank:

```json
{
  "hoeHit": 12,
  "spring1": { "bank": "Wave Bank(1)", "index": 3 },
  "coin": [40, { "bank": "Wave Bank", "index": 41 }]
}
```

Diagnostics (same `--content` handling as the game):

```sh
valley --content "<dir>" --audio-info              # banks, formats, how each cue resolves
valley --content "<dir>" --export-cue hoeHit out.wav
valley --content "<dir>" --audio-test hoeHit,coin  # play on the sound device
```

Playback uses cpal on Windows/macOS and ALSA on Linux (`libasound.so.2`,
loaded at runtime; PulseAudio/PipeWire provide the ALSA default device). If
no device can be opened the game logs one line and continues silently;
`--mute` (or `VALLEY_MUTE=1`) skips audio entirely and headless runs never
open a device. Music choice (title theme, `Data/Locations` music, rain,
seasonal tracks, silence at night) lives in `valley_game::music`.
