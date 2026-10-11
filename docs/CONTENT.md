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
real files inside this repository. Sprite-sheet layout assumptions can be
overridden without code changes by placing a `valley_layouts.json` at the
root of your content folder. Every field is optional; omitted fields keep
their defaults:

```json
{
  "character": { "frame_w": 16, "frame_h": 32, "rows": [0, 1, 2, 3], "walk_frames": 4 },
  "crops":     { "frame_w": 16, "frame_h": 32, "frames_per_crop": 8, "crops_per_row": 2 },
  "hoe_dirt":  { "texture": "TerrainFeatures/hoeDirt", "cols": 4, "watered_offset": [64, 0] },
  "trees":     { "texture": "TerrainFeatures/tree{kind}_{season}", "canopy": [0, 0, 48, 96], "stump": [32, 96, 16, 32] },
  "farmer":    { "texture": "Characters/Farmer/my_simple_sheet" },
  "hats":      { "texture": "Characters/Farmer/hats", "cell_w": 20, "cell_h": 20, "facing_rows": [0, 1, 3, 2], "offset": [-2, -5] },
  "clothing_icons": { "texture": "TileSheets/my_clothing_icons" }
}
```

* `farmer.layers`: compose the farmer from your own sheets, bottom first.
  Each layer copies, for every output frame (4 walk frames x 4 facings of
  16x32), the source cell at `origin + (cols[walk] * frame_w, rows[facing] *
  frame_h)`, places it at `offset` inside the frame, optionally mirrors it
  for facing left (`flip_left`) and multiplies it by an appearance colour
  (`tint`: `skin`, `hair`, `shirt`, `pants`). Example shape (values must be
  adapted to the real sheets):

  ```json
  "farmer": { "layers": [
    { "texture": "Characters/Farmer/farmer_base", "frame_w": 16, "frame_h": 32, "rows": [0, 2, 4, 2], "cols": [0, 1, 0, 2], "flip_left": true },
    { "texture": "Characters/Farmer/shirts", "frame_w": 8, "frame_h": 8, "rows": [0, 1, 3, 2], "cols": [0, 0, 0, 0], "offset": [4, 15], "tint": "shirt" },
    { "texture": "Characters/Farmer/hairstyles", "frame_w": 16, "frame_h": 32, "rows": [0, 1, 2, 1], "cols": [0, 0, 0, 0], "tint": "hair", "flip_left": true }
  ] }
  ```
* `hats`: each hat is one column of four cells (down, right, up, left in
  `facing_rows` order) and hats fill the sheet left to right; `offset` places
  the cell relative to the farmer sprite (art pixels). The values above are a
  starting point for a 20x20 hat sheet, not verified against real files.

* `character.rows` are the sheet rows for facing down, right, up and left.
* `trees.texture` substitutes `{kind}` (tree type id) and `{season}`.
* `farmer.texture` swaps the generated placeholder farmer for any sheet laid
  out like `character` (the original game composes its farmer from several
  layered sheets, which this engine does not attempt to replicate).

Other conventions:

| Area | Where |
|---|---|
| Object icons (16x16 grid, columns from texture width) | `valley_render::world_draw::object_sprite` |
| Big craftables (16x32 grid) | `valley_render::world_draw::big_sprite` |
| Farm fallbacks when map properties are missing (shipping bin, farmhouse door, bed) | `valley_sim::world::World::setup_compat` |
| Well-known item ids used for debris, ores, hay, bouquet, pendant, fertilizer, sprinklers | `valley_sim::{world, mines, romance, machines}` |
| Pet sheet (32x32 frames, rows down/right/up/left; `Animals/dog`, else the placeholder `Animals/Dev_Dog`), bowl from the `PetBowlTile` map property | `valley_sim::pets` |
| Trash cans: `Action Garbage <id>` tiles, loot from `Data/GarbageCans` (`GarbageCans`, `BeforeAll`, `AfterAll`, `BaseChance`) | `valley_sim::pets::search_garbage` |
| Achievements: an engine-defined list of original milestones (real achievement rules live in game code) | `valley_sim::pets::ACHIEVEMENTS` |
| Weapon special moves by `Type` (0/3 sword block, 1 dagger flurry, 2 club slam), slingshots are `Type` 4 | `valley_sim::arsenal` |
| Slingshot ammo damage: `Port.AmmoDamage` custom field on any object, else well-known stone/wood/ore/coal ids | `valley_sim::arsenal::ammo_damage` |
| Traveling cart: `TravelingCartTile` map property (top-left of a 3x2 footprint), Fridays and Sundays, `Traveler` shop or a generated stock | `valley_sim::cart` |
| Shop item queries: plain ids, `RANDOM_ITEMS <type> [min] [max] [@requirePrice] [@isRandomSale]` (objects, big craftables, furniture, clothing, boots, weapons, trinkets), `ALL_ITEMS [type]`, `DISH_OF_THE_DAY` (a random cooked dish per day), `RandomItemId`, `MaxItems`, `AvailableStock` (per day) | `valley_sim::shop` |
| Machine triggers: `ItemPlacedInMachine`, `MachinePutDown`, `DayUpdate`, `OutputCollected` (with the collected item as input); outputs: `ItemId`, `RandomItemId`, `OutputMethod` (only `OutputSeedMaker`: 1-3 seeds of the input crop, rare mixed/ancient seeds, approx.) | `valley_sim::machines` |
| Qi board: orders with `OrderType: Qi` on `SpecialOrders Qi` / `QiChallengeBoard`; `Gems` rewards add Qi gems | `valley_sim::special_orders` |
| Movie snacks: `Data/Concessions` (`Id`, `Name`, `DisplayName`, `Price`, `ItemTags`) and `Data/ConcessionTastes` (`Name` or `*`, `LovedTags`, `LikedTags`, `DislikedTags`); +50 / +25 friendship for loved / liked (approx.) | `valley_sim::movies` |
| Farm cave: location `FarmCave`; after 25,000g earned, entering it offers mushrooms (6 mushroom boxes: big craftable `128` or `Port.MushroomBox`, producing through their machine rules) or fruit bats (0-3 fruits a day, approx.) | `valley_sim::farm_cave` |
| Special orders: `Data/SpecialOrders` town orders (objectives Collect, Ship, Deliver, Fish, Gift, Slay, ReachMineFloor; rewards Money, Friendship, Mail; `RequiredTags` season_/mail_/event_/knows_), board via `Action SpecialOrders` | `valley_sim::special_orders` |
| Children: optional `CribTile` farmhouse map property (else a free tile near the bed), placeholder sheet `Characters/Dev_Child`; ask chance, stage lengths and names are engine constants | `valley_sim::children` |
| Mine start floor: `Action Mine <floor>`; floors past 120 are the deep cavern (no elevator, tougher monsters, shafts) | `valley_sim::mines` |
| Paid travel: `Action Port.Travel <location> <x> <y> <fare> [hhmm-hhmm]` (engine convention, 30 in-game minutes per trip) | `valley_sim::world::World::travel` |
| Fish ponds: buildings with id `Fish Pond` or the `Port.FishPond` custom field; `Data/FishPondData` (`RequiredTags`, `SpawnTime`, `ProducedItems`, `PopulationGates`, `MaxPopulation`) | `valley_sim::fish_ponds` |
| Kitchen fridge: `Action Fridge` tile; cooking uses backpack then fridge | `valley_sim::world::FRIDGE_TILE` |

| Clothing: `Data/Hats` (slash format), `Data/Shirts` / `Data/Pants` (`DefaultColor` tints the generated farmer), qualified ids `(H)`, `(S)`, `(P)` | `valley_sim::item::ClothingInfo` |
| Warp items: `Port.WarpTo` custom field ("<location> [x y]") on an object, or well-known totem ids (destination = the location's `DefaultArrivalTile`, approx.) | `valley_sim::world::World::warp_totem_target` |
| Buildings: `DefaultAction` runs as a tile action on non-door tiles; `ObeliskWarp <location> <x> <y>` teleports | `valley_sim::farm`, `World::teleport` |
| Secret notes: `Data/SecretNotes`; note scraps are object `79` or the placeholder `Dev_NoteScrap`, found while digging artifact spots and searching trash cans (approx. chance) | `valley_sim::secret_notes` |
| Stables: building id `Stable` or `Port.Stable` custom field; horse sheet `Animals/horse` (else placeholder `Animals/Dev_Horse`), 32x32 frames; riding speed x1.6 (approx.) | `valley_sim::horse` |
| Minecarts: `Data/Minecarts` networks (`UnlockCondition`, `LockedMessage`, `Destinations` with `Price`, `TargetLocation`, `TargetTile`), tile action `MinecartTransport [network] [exclude]` | `valley_sim::minecarts` |
| TV: weather, fortune and the cooking show from `Data/TV/CookingChannel` ("recipe/text"; Sundays, rerun Wednesdays) | `valley_sim::minecarts::Channel` |
| Shops: `Currency` (0 money, 1 star tokens, 2 Qi coins, 4 Qi gems) and `TradeItemId` / `TradeItemAmount` on items | `valley_sim::shop`, `World::buy` |
| Casks: big craftable `163` or `Port.Cask`; aging rate from `Port.AgingRate` or well-known drink/cheese ids (approx.); silver/gold/iridium at 14/28/56 base days | `valley_sim::casks` |
| Tailoring: `Data/TailoringRecipes` (`FirstItemTags`, `SecondItemTags`, `SpendRightItem`, `CraftedItemId(s)`); sewing machine = big craftable with `Port.SewingMachine` or a `Tailoring` tile action | `valley_sim::tailoring` |
| Fishing gear: rods hold bait from upgrade level 1 and tackle from level 2 (approx.); bait = category -21; tackle = category -22 with `Port.Tackle` (`bite_speed`, `bar_size`, `slow_escape`, `no_bounce`, `treasure`) or well-known ids; 20 uses | `valley_sim::fishing_gear` |
| Harvest huts: building id `Junimo Hut` or `Port.HarvestHut`; gathers ripe hand-harvested crops within 8 tiles each night into its storage | `valley_sim::harvest_huts` |
| Movies: `Data/Movies` (`Seasons`, `YearModulus`/`YearRemainder`, `Title`, `Tags`, `Scenes[].Text`), `Data/MoviesReactions` (`NPCName`, `Reactions[].Tag/Response`); tickets = object `809` or placeholder `Dev_Ticket`; theater door = `Theater` tile action | `valley_sim::movies` |
| Passive festivals: `Data/PassiveFestivals` (`Season`, `StartDay`/`EndDay`, `Condition`, `StartMessage`, `MapReplacements` location -> map asset); setup/cleanup methods are code in the original and not run | `valley_sim::passive_festivals` |
| Casino: original games "Lucky Reels" (`Port.Reels`) and "High-Low" (`Port.HighLow`) played with Qi coins; `Port.CoinCounter` sells 100 coins for 1000g; prize shops use `Currency: 2` | `valley_sim::casino` |
| Perfection: engine-defined weighted score (shipping, fish, cooking, crafting, friends, skills, museum, bundles) | `valley_sim::perfection` |
| Mastery (engine-defined): XP past level 10 counts toward 5 mastery levels; one perk per skill at a `Port.Mastery` tile | `valley_sim::mastery` |
| Books: objects with `Port.BookXp` ("<Skill> <xp>") and/or `Port.BookPower` (id in `Data/Powers`) | `valley_sim::mastery::read_book` |
| Trinkets: `Data/Trinkets` items `(TR)id`, worn in the trinket slot; effects via `Port.TrinketStats` ("attack 2 defense 1 speed 1 magnet 32 luck 1"); the original's effect classes are not implemented | `valley_sim::item::TrinketData` |
| Divorce: `Port.Divorce` / `DivorceBook` tile action, fee 50,000g (approx.); jealousy: -30 spouse points for gifts to another romanceable villager within 10 tiles of the spouse (approx.) | `valley_sim::romance` |
| Collectibles: `Port.Collectible` ("<counter> [n]") objects go to a counter (golden walnut `73` -> `GoldenWalnut`); `Port.Unlock <counter> <cost> <flag> [text]` spends them; Buildings tiles with `Port.ClearedBy <flag>` vanish once the flag is set | `valley_sim::island` |
| Slime hutches: building `Slime Hutch` or `Port.SlimeHutch` (up to 20 slimes, saved with the building); troughs = tiles with `Port.Trough` (else x 16, y 6-9); each watered trough may grow a slime ball (big craftable from the building's `Port.SlimeBall`, default `56`; approx. 25% per slime) | `valley_sim::slime_hutch` |
| Slime incubators: big craftable `156` or `Port.SlimeIncubator`, hutch only; eggs = `Port.SlimeType <monster>` or well-known egg ids; 4000 minutes; slime balls drop `766` (or `Port.SlimeBallDrop`) and sometimes `557` | `valley_sim::slime_hutch` |
| Cabins: building `Cabin` or `Port.Cabin`; each farmhand claims a free cabin (else shares the farmhouse) and wakes beside its `Bed` tile | `valley_sim::multiplayer` |
| Arcade cabinets: tile actions `Arcade_Prairie` (Dust Ranger) and `Arcade_Minecart` (Rail Runner), `Port.Arcade <ranger|rails>`, or big craftables with that custom field; both games are original designs; best scores in counters `Arcade.<id>.Best` / `.Wins` | `arcade`, `valley_sim::evaluation::arcade_finished` |
| Events: `Data/Events/<location>` scripts; see `valley_sim::events` for the supported commands (code-defined ones like `specificTemporarySprite` are skipped); `end dialogue <npc> "<text>"` sets that villager's next line; emotes from `TileSheets/emotes` (16x16, four frames per row, row = emote id / 4) | `valley_sim::events` |
| Dialogue: `$c`, `$d <joja|cc|bus|kent|flag>`, `$p <ids>`, `$1 <id>` (once-only, remembered as `Port_Dialogue_<id>`), `$k`, `[item ids]` gifts, `%spouse %pet %kid1 %kid2 %year %season %time %favorite`, `%adj/%noun/%place/%name` from small built-in lists; `Characters/Dialogue/rainy` on rainy days | `valley_sim::dialogue`, `World::prepare_dialogue` |
| Schedules: key fallbacks `<season>_<day>`, `<day>_<hearts>`, `<day>`, `rain2` (50%), `rain`, `<season>_<weekday>_<hearts>`, `<season>_<weekday>`, `<weekday>`, `<season>`, `spring_<weekday>`, `spring`, `default`; `NOT friendship` falls back to `default`; `a<time>` leaves early (approx. walk time) | `valley_sim::schedule` |
| Grandpa's evaluation: first morning of year 3 (21-point public table), candles drawn on the `Port.Shrine` tile; a diamond (`72`) on the shrine re-evaluates next morning; 4 candles give big craftable `160` | `valley_sim::evaluation` |
| Joja route: `Port.JojaMembership` (5,000g, flag `JojaMember`, closes bundles), `Port.JojaForm` funds projects from `Port/JojaProjects` (`Id`, `Area`, `DisplayName`, `Price`, `Flag`; default: the original five areas and prices) | `valley_sim::evaluation` |
| Area flags: finishing a bundle area (or its Joja project) sets `ccDone_<area>`, the original flag for known areas (`ccPantry`, `ccCraftsRoom`, `ccFishTank`, `ccBoilerRoom`, `ccVault`, `ccBulletin`) and `ccIsComplete` when all are done | `valley_sim::evaluation::complete_area` |
| Resource clumps: 2x2 stumps (copper axe), logs (steel axe), boulders (steel pickaxe), meteorites (gold pickaxe), mine boulders, drawn from `Maps/springobjects` at sheet indices 600/602/672/622/752-758; health and drops approx.; random ones on a new farm unless the farm map has `Port.NoClumps` | `valley_sim::clumps` |
| Giant crops: `Data/GiantCrops` (`FromItemId`, `HarvestItems`, `Texture`, `TexturePosition`, `TileSize`, `Health`, `Chance`, `Condition`); a square of ripe crops may merge overnight in outdoor locations | `valley_sim::clumps` |
| Crows and scarecrows: crows need 16+ farm crops (30% per 16, max 4 a night); scarecrows are big craftables tagged `crow_scare` (radius `crow_scare_radius_<n>`, default 9) or named "...arecrow"; crow sprite `LooseSprites/Port_Crow` (4 frames of 16x16) | `valley_sim::farm_events` |
| Night events (approx. 1% each): fairy (grows crops within 3 tiles), witch (void egg `305` in a coop), meteorite, stone owl (big craftable `95`), strange capsule (`96`, year 2+); summer 3 of year 1 sets the `Port_Earthquake` flag (pair with `Port.ClearedBy`) | `valley_sim::farm_events` |
| Weather totems: `Port.SetWeather <Rain|Storm|Sun|Snow|Wind>` on an object, or the rain totem `681`; refused when tomorrow is a festival | `valley_sim::farm_events` |
| Volcano (generic): `Port.Volcano` enters 5 generated floors with lava (`Type: Lava` tiles burn), shard-bearing rocks and a `Port.Forge` on the last floor; shards = object `848` or placeholder `Dev_EmberShard`; +3 damage per forge level (approx.) | `valley_sim::volcano` |

## Mods

`--mods <dir>` / `VALLEY_MODS` loads every subfolder of `<dir>` as a mod
(`valley_content::mods`):

* Files mirror the Content layout (`Data/Objects.json`, `Maps/Farm.tmx`,
  `TileSheets/crops.png`, ...).
* JSON assets are merged into the base content: object keys are added or
  replaced, `null` removes a key, array entries with the same `Id` are
  replaced and others appended. Every other file type replaces the original.
* `manifest.json` (optional): `Name`, `Version`, `Author`, `Description`,
  `Priority` (higher loads later and wins), `Enabled`.
* Mods load in (`Priority`, folder name) order; loaded mods are listed on
  start-up.
