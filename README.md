# valley-rs

A clean-room Rust engine that plays a farming-life game using **your own copy**
of the game's content files, designed as a multi-game host so other Rust game
ports can be merged in as modules.

* No original game code is used: every system is written from scratch from
  publicly documented mechanics and observed behaviour.
* No original game assets are in this repository. At runtime the engine reads
  textures, maps and data from a folder you point it at (your own game's
  `Content` folder, unpacked to PNG/JSON/TMX with StardewXNBHack).
* Without a content folder it runs on a small, original placeholder pack so it
  is always runnable and testable.

## Layout

| crate | purpose |
|---|---|
| `engine` | Game-agnostic core: textures, depth-sorted draw lists, input, RNG, audio queue, `GameModule` trait + `ModuleHost` (multi-game hosting), software renderer. |
| `valley_content` | Content pipeline: TMX/TBIN/XNB map + texture readers, JSON data models, string tokens, layered sources, original placeholder pack. |
| `valley_sim` | Headless game simulation (time, locations, farmer, items, tools, crops, NPCs, saves). |
| `valley_render` | Turns simulation state into draw commands. |
| `valley_app` | Window (macroquad) + headless runner/screenshot tool. |

See `docs/` for architecture, content setup and progress.
