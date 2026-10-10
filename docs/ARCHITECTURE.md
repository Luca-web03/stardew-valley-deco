# Architecture

```
valley_app ──► engine::ModuleHost ──► [GameModule] valley_game ──► valley_render ──► engine::DrawList
                     ▲                         │                         │
                     │                         ▼                         ▼
              other game modules          valley_sim (World)       engine::TextureStore
                                               │
                                               ▼
                                        valley_content (ContentManager: your files ▸ placeholder pack)
```

## engine (game-agnostic)

* `GameModule` trait (`init`, fixed-tick `update`, `draw`, `suspend`/`resume`).
* `ModuleHost` runs one active module at 60 ticks/s and can host many:
  this is the merge point for other games ported to Rust. A module switches
  to another by setting `ctx.switch_to = Some("id")`; Ctrl+Tab cycles.
* Games never touch a graphics API: they fill a depth-sorted `DrawList`
  (sprites with source rect / scale / flip / rotation / tint, and fills).
  The window backend (macroquad) and the software renderer (`engine::soft`,
  used for headless screenshots and tests) both consume it.
* `InputState` (held/pressed/released, mouse, wheel, text), `AudioQueue`,
  deterministic `Rng`, built-in bitmap font.

### Merging another game

Implement `GameModule` for it (or wrap its loop), then in `valley_app`:

```rust
host.add(Box::new(valley_game::ValleyGame::new(cm, saves)));
host.add(Box::new(my_other_game::Game::new()));
```

Shared services (textures, input, audio) live in `EngineCtx`, so modules can
also exchange data by agreeing on texture names or by holding shared state
(`Rc<RefCell<..>>`) created in `main`.

## valley_content

Readers for PNG, JSON, TMX (Tiled), TBIN (xTile binary) and simple XNB; typed
data models (`Data/Objects`, `Data/Crops`, `Data/Characters`, `Data/Shops`,
`Data/Locations`, `Data/Tools`); tokenized-string expansion; a layered source
(your content folder over the built-in placeholder pack).

## valley_sim

Pure, deterministic simulation; no graphics. `World::update(&PlayerInput)`
advances one tick and emits `WorldEvent`s (sounds, dialogue, shop, warps,
day end...). Saves are JSON snapshots of everything not reloaded from content.

## valley_render / valley_game

`valley_render` converts world state into draw commands (depth bands
documented in its crate docs). `valley_game` owns screens, menus, HUD, input
mapping and saving, and implements `GameModule`.
