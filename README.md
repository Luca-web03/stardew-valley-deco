# valley-rs

A clean-room Rust engine for a farming-life game, built to run on **your own
copy** of the original game's content and to host other Rust game ports as
modules in the same executable.

* **No original code.** Every system is written from scratch from publicly
  documented mechanics and observed behaviour. Where an exact value isn't
  public, the code uses a clearly marked approximation (see `docs/PROGRESS.md`).
* **No original assets in this repository.** At runtime the engine reads
  textures, maps, data and audio from a folder you point it at (see
  `docs/CONTENT.md`). Without one it runs on a small, original placeholder
  pack, so it is always playable and testable.

## Run

```sh
# Placeholder content
cargo run --release -p valley_app

# Your own unpacked game content
cargo run --release -p valley_app -- --content "/path/to/Content (unpacked)"

# Skip the title screen
cargo run --release -p valley_app -- --new Robin --seed 42

# Headless: scripted input + screenshot (no window, no audio device)
cargo run -p valley_app -- --headless --new Robin \
    --script "cmd:mine_3 wait:60 shot:out/mine.png"
```

`valley --help` lists every option and the script language.

## Controls (default, rebindable in Menu → Options)

| Action | Keys |
|---|---|
| Move | WASD / arrows (hold Shift to walk) |
| Use tool / item | C or left click (hold to charge upgraded tools, reel fish) |
| Check, talk, harvest, eat, place | X or right click |
| Toolbar slot | 1–0, -, =, mouse wheel |
| Menu (items, crafting, skills, social, options) | E / I / Esc |
| Journal (quests) | F |
| Console | F1 (commands: `warp`, `mine`, `give`, `money`, `time`, `day`, `slot`, `tile`, `bundles`) |
| Debug overlay | F3 |
| Switch hosted game | Ctrl+Tab |

## What's in the game

Farming (tilling, watering, crops, fertilizer, sprinklers), foraging,
trees and debris, tools and upgrades, crafting and machines, cooking and
buffs, fishing minigame, mines with combat, farm buildings and animals,
villagers with schedules, dialogue, gifts, dating and marriage, shops,
mail, quests, bundles, museum, cutscene events, festivals, skills and
professions, weather and seasons, day/night, saves, audio. See
`docs/PROGRESS.md` for the detailed status.

## Workspace

| crate | purpose |
|---|---|
| `engine` | Game-agnostic core: textures, depth-sorted draw lists, input, RNG, audio queue + mixer + synth, `GameModule` trait and `ModuleHost` (multi-game hosting), software renderer. |
| `valley_content` | Content pipeline: TMX/TBIN/XNB maps and textures, XACT audio banks, JSON data models, string tokens, layered sources, original placeholder pack. |
| `valley_sim` | Headless, deterministic game simulation (everything gameplay). |
| `valley_render` | Turns simulation state into draw commands; UI primitives. |
| `valley_game` | The playable module: screens, menus, HUD, input mapping, saves, music. |
| `valley_app` | Desktop window (macroquad) + headless runner + audio output. |

## Mods

Point `--mods <dir>` (or `VALLEY_MODS`) at a folder of mods. Each subfolder
mirrors the Content layout and is layered over the game content:

```
Mods/
  BetterTurnips/
    manifest.json          {"Name": "Better Turnips", "Version": "1.0", "Priority": 0}
    Data/Objects.json      {"Dev_Turnip": {...}, "Dev_Driftwood": null}
    Animals/Dev_Dog.png    replaces the original image
```

JSON assets merge entry by entry (a `null` value removes an entry; array
entries match by `Id`), so mods only need the entries they change. Images,
maps and audio replace the original file. See `docs/CONTENT.md`.

## Merging other games

Each game is an `engine::GameModule`. Add yours next to the valley module in
`crates/valley_app/src/main.rs` (`host.add(Box::new(MyGame::new()))`); the
player switches with Ctrl+Tab, or a module hands over with
`ctx.switch_to = Some("other_id".into())`. Shared services (textures, input,
audio) live in `EngineCtx`. Details in `docs/ARCHITECTURE.md`.

## Tests

```sh
cargo test --workspace
```

The simulation tests play through whole days, farming loops, fishing,
mining, festivals, marriage and more on the placeholder pack.
