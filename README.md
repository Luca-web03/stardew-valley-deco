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

# Co-op: host, then join from another machine (the client needs no content)
cargo run --release -p valley_app -- --new Robin --host 24642
cargo run --release -p valley_app -- --join 192.168.1.20:24642 --name Sam

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
| Console | F1 (commands: `warp`, `mine`, `give`, `money`, `time`, `day`, `child`, `wear`, `slot`, `tile`, `bundles`, `clump`, `night`, `host`) |
| Weapon special move | X or right click with a weapon held (nothing else to interact with) |
| Slingshot | hold C / left mouse to pull, release to fire (aims at the mouse) |
| Debug overlay | F3 |
| Switch hosted game | Ctrl+Tab |

## What's in the game

Farming (tilling, watering, crops, fertilizer, sprinklers, giant crops,
crows and scarecrows), foraging, trees, debris and resource clumps, tools and upgrades, crafting and machines, cooking and
buffs, a kitchen fridge, fishing minigame, crab pots and fish ponds, mines
with combat, weapon special moves, a slingshot and a deep cavern with
shafts, farm buildings and animals, a farm pet, villagers with schedules,
dialogue, gifts, dating, marriage and children, shops, a traveling cart,
clothing, mail, quests, special orders, bundles, museum, secret notes,
achievements, trash cans, cutscene events, festivals, skills and
professions, warp totems, obelisks and paid travel, weather and seasons,
day/night, saves, audio, content mods and online co-op. See `docs/PROGRESS.md` for the
detailed status.

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

## Co-op

The host runs the whole game; joined players are farmhands with their own
farmer, inventory, location and menus, and wake up in the farmhouse each
morning. The host streams compressed draw lists, textures and sounds to each
client, and clients send back raw input, so a client only needs the
executable. The day ends when the host sleeps; the game doesn't pause for
menus while farmhands are connected. Farmhands are saved with the host's game.

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
