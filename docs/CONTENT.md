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
textures in Color, Bgr565, Bgra5551, Bgra4444 and DXT1/3/5 formats, sprite
fonts and maps); LZX-compressed files need unpacking first.

Text uses the game's `Fonts/SmallFont` when present (XNB, or an unpacked
`Fonts/SmallFont.json` + `.png` pair: `LineSpacing`, `Spacing`,
`DefaultCharacter`, `Glyphs` { char: { `BoundsInTexture`, `Cropping`,
`LeftSideBearing`, `Width`, `RightSideBearing` } }), laid out like XNA's
`SpriteFont` and scaled so a line advances as far as the built-in 8x8 font
(the fallback). The language (options tab, saved in `options.json`) makes
assets named `<asset>.<code>` win (`de-DE`, `es-ES`, `fr-FR`, `hu-HU`,
`it-IT`, `ja-JP`, `ko-KR`, `pt-BR`, `ru-RU`, `tr-TR`, `zh-CN`, plus
`Data/AdditionalLanguages` `LanguageCode`s); text already loaded keeps its
language until restart. Non-Latin scripts need the matching game font.

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
  "clothing_icons": { "texture": "TileSheets/my_clothing_icons" },
  "ui":        { "texture": "Maps/MenuTiles", "box": [0, 256, 60, 60], "slot_tile": 10, "tile_size": 64, "scale": 1.0 },
  "hud":       { "texture": "LooseSprites/Cursors", "clock_box": [333, 431, 71, 43], "energy_frame": [256, 408, 14, 42],
                 "health_frame": [268, 408, 14, 42], "bar_inset": [3, 13, 3, 2], "scale": 4.0 }
}
```

* `hud`: with that texture the day/time box and the energy/health bar
  frames come from it (`bar_inset`: left, top, right, bottom of the fill
  area in sheet pixels); the money counter sits under the box. Rects are
  assumptions to adjust against the real sheet.

* `ui`: menu frames are a 9-slice of `box` (corners a third of its width,
  drawn at `scale`; buttons at half) and inventory slots are tile
  `slot_tile` of a `tile_size` grid in `texture`. Without that texture the
  menus use flat colours.

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
| Pets: `Data/Pets` types (`DisplayName`, `BarkSound`, `ContentSound`) and `Breeds` (`Id`, `Texture`, `CanBeChosenAtStart`), chosen on the new-game screen; sheets are 32x32 frames, rows down/right/up/left (fallback `Animals/dog`, else the placeholder); bowl from the `PetBowlTile` map property; gifts: each morning `GiftChance` (default 0.2) to leave one of the `Gifts` (`QualifiedItemID`, `Stack`, `MinimumFriendshipThreshold`, `Weight`) on a free tile beside the pet | `valley_sim::pets` |
| Trash cans: `Action Garbage <id>` tiles, loot from `Data/GarbageCans` (`GarbageCans`, `BeforeAll`, `AfterAll`, `BaseChance`) | `valley_sim::pets::search_garbage` |
| Achievements: `Data/Achievements` names with conditions for the original ids 0-31 and 34 from the public list (earnings, friends, recipes, house, crafting, fish, museum, requests, crops, family, skills, shipping); without that data, an engine-defined list | `valley_sim::achievements` |
| Weapon special moves by `Type` (0/3 sword block, 1 dagger flurry, 2 club slam), slingshots are `Type` 4 | `valley_sim::arsenal` |
| Slingshot ammo damage: `Port.AmmoDamage` custom field on any object, else well-known stone/wood/ore/coal ids | `valley_sim::arsenal::ammo_damage` |
| Traveling cart: `TravelingCartTile` map property (top-left of a 3x2 footprint), Fridays and Sundays, `Traveler` shop or a generated stock | `valley_sim::cart` |
| Shop item queries: plain ids, `RANDOM_ITEMS <type> [min] [max] [@requirePrice] [@isRandomSale]` (objects, big craftables, furniture, clothing, boots, weapons, trinkets), `ALL_ITEMS [type]`, `DISH_OF_THE_DAY` (a random cooked dish per day), `RandomItemId`, `MaxItems`, `AvailableStock` (per day) | `valley_sim::shop` |
| Machine triggers: `ItemPlacedInMachine`, `MachinePutDown`, `DayUpdate`, `OutputCollected` (with the collected item as input); outputs: `ItemId`, `RandomItemId`, `OutputMethod` (only `OutputSeedMaker`: 1-3 seeds of the input crop, rare mixed/ancient seeds, approx.) | `valley_sim::machines` |
| More machine fields: `ReadyTimeModifiers`, `ExperienceGainOnHarvest`, `PreventTimePass` (Outside/Inside/season/Sun/Rain/Always), `OnlyCompleteOvernight`, `InvalidCountMessage` (rule or machine; `{0}` = count), output `StackModifiers`/`QualityModifiers` (modifier `Condition`s see the input item); output methods `OutputGeodeCrusher` (geode contents), `OutputDeconstructor` (a crafted item's main ingredient), `OutputMushroomLog` (approx. mushroom pool) | `valley_sim::machines` |
| `FLAVORED_ITEM <kind> <source>` kinds: Wine 348, Juice 350, Jelly 344, Pickle 342, Honey 340, Roe 812, AgedRoe 447, SmokedFish, DriedFruit, DriedMushrooms, SpecificBait (prices approx.); source `-1` makes the plain item ("Wild Honey"). Honey collected from a machine takes the flavor of the closest fully grown flower crop (category -80) within 5 tiles, price 100 + 2x flower | `valley_sim::machines` |
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
| Monster species: picked from the `Data/Monsters` key by keywords (crab, duggy/mole, skeleton, mage, shaman, squid/lurk/rex/spitter, mummy, hot head/bomber, big slime/big blob, grub/larva, dust); mine floors spawn the original names when present | `valley_sim::monster_kinds`, `valley_sim::mines::spawn_table` |
| Farm types: maps `Maps/Farm`, `Farm_Fishing`, `Farm_Foraging`, `Farm_Mining`, `Farm_Combat`, `Farm_FourCorners`, `Farm_Island`, `Farm_Ranching` (offered when present) and `Data/AdditionalFarms` (`ID`, `MapName`, `TooltipStringPath`); `Data/Locations` `Farm_<suffix>` replaces the farm's fish/forage/artifact data; Hill-top quarry from `Port.MiningArea "x y w h"` | `valley_sim::farm_types` |
| Festival activities: by festival key (spring13 egg hunt, spring24 dance, summer11 soup, fall16 grange, winter8 ice fishing, winter25 gift exchange) or `Port.Activity` in the festival data; `Port.Prize` names the prize; the host offers soup/grange before the main event; secret friends are named by letter on winter 18 | `valley_sim::festival_activities` |
| World map: `Data/WorldMap` (`BaseTexture`, `MapAreas` with `PixelArea`, `Textures`, `Tooltips`, `WorldPositions` `LocationName(s)`/`TileArea`/`MapPixelArea`, conditions); opened with M | `valley_sim::world_map` |
| Grandpa's evaluation: first morning of year 3 (21-point public table), candles drawn on the `Port.Shrine` tile; a diamond (`72`) on the shrine re-evaluates next morning; 4 candles give big craftable `160` | `valley_sim::evaluation` |
| Joja route: `Port.JojaMembership` (5,000g, flag `JojaMember`, closes bundles), `Port.JojaForm` funds projects from `Port/JojaProjects` (`Id`, `Area`, `DisplayName`, `Price`, `Flag`; default: the original five areas and prices) | `valley_sim::evaluation` |
| Remixed bundles (new-game option when `Data/RandomBundles` exists): per area `AreaName`, `Keys` (bundle keys in slot order), `BundleSets` (one picked; bundles placed by `Index`) and `Bundles` (pool for the free slots, preferring the same `Index`); each bundle `Name`, `Color` (name or number), `Sprite`, `Items` ("count [NQ/SQ/GQ/IQ] name-or-id", comma-separated; `Money` = gold), `Pick`, `RequiredItems`, `Reward` ("count name", "count BO id" or "O id n"). The roll is stored in the save; bundle strings read the display name from field 6 or 5 | `valley_sim::bundle_remix` |
| Area flags: finishing a bundle area (or its Joja project) sets `ccDone_<area>`, the original flag for known areas (`ccPantry`, `ccCraftsRoom`, `ccFishTank`, `ccBoilerRoom`, `ccVault`, `ccBulletin`) and `ccIsComplete` when all are done | `valley_sim::evaluation::complete_area` |
| Resource clumps: 2x2 stumps (copper axe), logs (steel axe), boulders (steel pickaxe), meteorites (gold pickaxe), mine boulders, drawn from `Maps/springobjects` at sheet indices 600/602/672/622/752-758; health and drops approx.; random ones on a new farm unless the farm map has `Port.NoClumps` | `valley_sim::clumps` |
| Giant crops: `Data/GiantCrops` (`FromItemId`, `HarvestItems`, `Texture`, `TexturePosition`, `TileSize`, `Health`, `Chance`, `Condition`); a square of ripe crops may merge overnight in outdoor locations | `valley_sim::clumps` |
| Crows and scarecrows: crows need 16+ farm crops (30% per 16, max 4 a night); scarecrows are big craftables tagged `crow_scare` (radius `crow_scare_radius_<n>`, default 9) or named "...arecrow"; crow sprite `LooseSprites/Port_Crow` (4 frames of 16x16) | `valley_sim::farm_events` |
| Night events (approx. 1% each): fairy (grows crops within 3 tiles), witch (void egg `305` in a coop), meteorite, stone owl (big craftable `95`), strange capsule (`96`, year 2+); summer 3 of year 1 sets the `Port_Earthquake` flag (pair with `Port.ClearedBy`) | `valley_sim::farm_events` |
| Weather totems: `Port.SetWeather <Rain|Storm|Sun|Snow|Wind>` on an object, or the rain totem `681`; refused when tomorrow is a festival | `valley_sim::farm_events` |
| Volcano (generic): `Port.Volcano` enters 5 generated floors with lava (`Type: Lava` tiles burn), shard-bearing rocks and a forge (`Port.Forge` or `Forge` action) on the last floor; shards = object `848` or placeholder `Dev_EmberShard` | `valley_sim::volcano` |
| Fences (`Data/Fences`: `Health`, `RepairHealthAdjustmentMinimum/Maximum`, `Texture`, `PlacementSound`, `RemovalSound`, `RemovalToolIds`, `RemovalToolTypes`): placed objects joining fence neighbours, -1 health a day outdoors, fallen (passable) at 0; gates = `325` or `Port.Gate` (open/close, no wear). Sheet layout from `valley_layouts.json` `fences` (default: 16x32 frames per neighbour mask N=1 E=2 S=4 W=8 in 4 columns, gate frames 16/17; placeholder layout, to verify against the real sheets) | `valley_sim::fences` |
| Floors and paths (`Data/FloorsAndPaths`: `ItemId`, `Texture`, `Corner`, `WinterTexture`, `PlacementSound`, `RemovalSound`, `FootstepSound`, `FarmSpeedBuff`): terrain placed from the item, objects may stand on it, pickaxe/axe lifts it, farm speed bonus (default 0.1, approx.). Tiles per neighbour mask from `Corner` (`valley_layouts.json` `flooring`, default 4 columns of 16x16; placeholder layout) | `valley_sim::fences` |
| Bushes: small/medium/large (1/2/3 tiles wide) and tea; medium bushes carry berries spring 15-18 (`296`) and fall 8-11 (`410`) (placeholder `Dev_Berry`), 1-2 + foraging level/4 per shake (approx.), iridium with Botanist; tea sapling (`251`/`Dev_TeaSapling`) grows in 20 days, leaves (`815`/`Dev_TeaLeaves`) on days 22-28 (not winter outdoors); farm bushes cut with an axe (small any, medium copper, large steel; approx.). Sheet layout: `valley_layouts.json` `bushes` | `valley_sim::bushes` |
| `Paths` map layer spawns: `Port/PathsLayer` maps a sheet-image key (substring, longest wins) and tile index to `Tree <type>` or `Bush <size>`; default: sheet `paths` 9/10/11 = wild trees 1/2/3 (other original indices unverified) | `valley_sim::bushes::spawn_paths_layers` |
| Automation: auto-grabber (`165`/`Port.AutoGrabber`, a chest gathering floor produce and tool produce in its building each morning), auto-petter (`272`/`Port.AutoPetter`), mini shipping bin (`248`/`Port.MiniShippingBin`, ships the held item), workbench (`208`/`Port.Workbench`, crafting also uses the 8 surrounding chests), pressure nozzle (`915`/`Port.Nozzle`: cross -> 3x3 -> 5x5 -> 7x7) and enricher (`913`/`Port.Enricher`: holds fertilizer, spreads it on unfertilized watered soil) | `valley_sim::automation` |
| Wallpaper and flooring: items `(WP)<n>` / `(FL)<n>` from `Maps/walls_and_floors` (112 / 56) or `Data/AdditionalWallpaperFlooring` sets (`Id`, `Texture`, `IsFlooring`, `Count`; ids `<Id>:<n>`); used in a decoratable location (FarmHouse*, Cabin*, *Shed*, or map property `Port.Decoratable`) they paint the clicked room (connected floor tiles; walls = up to 3 `Buildings` tiles above, fixtures with actions excluded). Sheet layout `valley_layouts.json` `decor` (default strips 16x48 x16 per row, floors 32x32 x8 per row from y 336; assumed) | `valley_sim::decor` |
| Swimming: `TouchAction` `ChangeIntoSwimsuit` / `ChangeOutOfSwimsuit` / `PoolEntrance` (needs the swimsuit; swimmers move only through water and entrances); spa pools (map property `Port.Spa` or location `BathHouse_Pool`) restore 1 energy and health per 200 ms (approx.) | `valley_sim::swimming` |
| Telephone: big craftable `214` or `Port.Telephone`; contacts from `Port/PhoneContacts` (`Name`, `Message`, `Kind`: `ToolUpgrade`, `Construction`, `DishOfTheDay`, `Text`), default blacksmith/carpenter/saloon | `valley_sim::phone` |
| New-game objects: map property `Port.StartObjects` ("bigCraftableId x y ...") | `valley_sim::world` |
| Game state queries: date (`SEASON`, `SEASON_DAY`, `DAY_OF_MONTH`, `DAY_OF_WEEK`, `DAYS_PLAYED`, `TIME`, `YEAR`, `IS_FESTIVAL_DAY`, `IS_PASSIVE_FESTIVAL_TODAY`, `IS_GREEN_RAIN_DAY`), weather/location (`WEATHER`, `LOCATION_NAME`, `LOCATION_IS_INDOORS/OUTDOORS`, `LOCATION_IS_MINES/SKULL_CAVE`, `LOCATION_ACCESSIBLE`, `LOCATION_CONTEXT`, `LOCATION_SEASON`), world (`FARM_TYPE`, `FARM_NAME`, `FARM_CAVE`, `IS_COMMUNITY_CENTER_COMPLETE`, `IS_JOJA_MART_COMPLETE`, `BUILDINGS_CONSTRUCTED`, `MUSEUM_DONATIONS`, `WORLD_STATE_ID`...), player (`PLAYER_*_LEVEL`, money, mail/flags, events, items, recipes, professions (names or original numbers), friendship/hearts, relationships, children, pet, stats, visited locations, special orders, secret notes, trigger actions run), randomization (`RANDOM [@addDailyLuck]`, `SYNCED_RANDOM`, `SYNCED_DAY_RANDOM`, `SYNCED_CHOICE`, `SYNCED_SUMMER_RAIN_RANDOM`), items (`ITEM_ID`, `ITEM_ID_PREFIX`, `ITEM_CATEGORY`, `ITEM_TYPE`, `ITEM_OBJECT_TYPE`, `ITEM_PRICE`, `ITEM_QUALITY`, `ITEM_STACK`, `ITEM_EDIBILITY`, `ITEM_CONTEXT_TAG` with `Target`/`Input`), `ANY`, `TRUE`, `FALSE`. Unknown queries fail (reported once) | `valley_sim::gsq` |
| Trigger actions (`Data/TriggerActions`: `Id`, `Trigger` DayStarted/DayEnding/LocationChanged, `Condition`, `Action`/`Actions`, `MarkActionApplied`) and actions (`AddMail`, `RemoveMail`, `AddMoney`, `AddItem`, `RemoveItem`, `AddQuest`, `RemoveQuest`, `AddSpecialOrder`, `RemoveSpecialOrder`, `AddFriendshipPoints`, `AddConversationTopic`, `RemoveConversationTopic`, `AddBuff` (object buffs), `RemoveBuff`, `IncrementStat`, `If ## ##`, `Mark*` (event seen, question answered, recipes, song heard, action applied), `SetNpcInvisible`/`SetNpcVisible`, `Null`); also run by event `action`, mail `%action ... %%` and dialogue `$action`; dialogue `$query <q>#yes|no` | `valley_sim::trigger_actions` |
| Location fish (`Data/Locations` `Fish`): `ItemId`/`RandomItemId`, `Season`, `Chance`, `Condition`, `FishAreaId` (with `FishAreas` `Position`), `BobberPosition`, `PlayerPosition`, `MinFishingLevel`, `Min/MaxDistanceFromShore`, `ApplyDailyLuck`, `CatchLimit`, `SetFlagOnCatch`, `RequireMagicBait` (bait `908`), `Precedence`, `IgnoreFishDataRequirements`, inheritable `Default` entries (`CanBeInherited`) | `valley_sim::fishing::choose_fish_at` |
| More `Data/Locations` fields: forage `RandomItemId` and `Condition`; artifact spots `RandomItemId`, `Condition`, `Precedence`, `ContinueOnDrop`, `ApplyGenerousEnchantment` plus the shared `Default` drops; `CanPlantHere` (default farm/greenhouse); `MinDailyWeeds`/`MaxDailyWeeds` (farm); `ChanceForClay` (hoeing; clay `330`) | `valley_sim::{machines, orchard, tools, world}` |
| Location contexts (`Data/LocationContexts`): the `Default` context's `WeatherConditions` (first passing `Condition`, checked on tomorrow's date) set tomorrow's weather; `AllowRainTotem`; a location's context is its map property `LocationContext`. Without them the engine's seasonal chances apply (approx.) | `valley_sim::location_contexts` |
| Crops: `IsRaised` (trellis crops block the way), `IsPaddyCrop` (no watering and +25% growth within 3 tiles of water; approx.); agriculturist +10% growth. Profession effects: pirate (2x treasure), mariner (no crab pot trash), luremaster (no bait), lumberjack (25% hardwood from felled trees, approx.), geologist (50% gem pairs, approx.), blacksmith (+50% bars), acrobat (special cooldown /2), desperado (2x crit damage), coopmaster/shepherd (more friendship from petting, approx.) | `valley_sim::{world, equipment, fishing, tools, mines, arsenal, farm}` |
| Mine gem nodes (`8`/`10` amethyst/topaz from level 1, `14`/`6` aquamarine/jade from 40, `12`/`4`/`2` emerald/ruby/diamond from 80; placeholder `Dev_GemNode`), ~1.5% of rocks (approx.) | `valley_sim::mines::GEM_NODES` |
| Mine treasure chests on floors 10-120 (public reward list: boots 506/509/511/512, weapons 11/32/22/33/8, stardrop 434, skull key flag `HasSkullKey`; placeholders otherwise), once per floor; `Port/MineChests` ({ "level": "qualified id" }) overrides | `valley_sim::mines::MINE_CHESTS` |
| `Data/Characters` `Appearance` (by `Season`, `Indoors`/`Outdoors`, `Condition`, lowest `Precedence` then `Weight`; `Sprite`/`Portrait` assets; island attire skipped) picked each morning, worn by where the NPC is; `UnlockConditions` hide NPCs while false | `valley_sim::npc` |
| `Data/Buffs` (`DisplayName`, `Duration` ms with -2 = rest of day, `Effects`, `IsDebuff`): object `Buffs` with a defined `BuffId` start from it (plus `CustomAttributes`); `AddBuff <id> [ms]` | `valley_sim::buffs` |
| `Data/Objects` `GeodeDrops` (precedence, chance, condition, stacks; `GeodeDropsDefaultItems`) for geodes and the geode crusher; `ArtifactSpotChances` (location -> chance to dig the object up); `ExcludeFromRandomSale` | `valley_sim::{progress, orchard, shop}` |
| More `Data/FarmAnimals` fields: `AlternatePurchaseTypes` (random breed among `AnimalIds` with a passing `Condition`), deluxe produce by `DeluxeProduceMinimumFriendship` / `DeluxeProduceCareDivisor` / `DeluxeProduceLuckMultiplier`, `ProfessionForFasterProduce` (one day less), `ProfessionForQualityBoost` (approx.), `HarvestedTexture` | `valley_sim::farm` |
| More `Data/Shops` fields: shop and item `PriceModifiers` (conditions see the item as `Target`), `IgnoreShopPriceModifiers`, `PerItemCondition`, `AvoidRepeat`, `Quality`, `ActionsOnPurchase`, `IsRecipe` (teaches the recipe); `SalableItemTags` decide what a shop buys (none: it buys nothing) | `valley_sim::shop` |
| Buildings, more: `BuildingToUpgrade` (upgrade rebuilds the base building in place over `BuildDays`, keeping its interior contents and animals; `MagicalConstruction` finishes at once), `HayCapacity` (silo 240 if absent), `Chests` (`Id`, `Type` Load/Collect, `Sound`, `InvalidItemMessage`) with overnight `ItemConversions` (`RequiredTags`, `RequiredCount`, `MaxDailyConversions`, `SourceChest`, `DestinationChest`, `ProducedItems`), `ActionTiles` (`BuildingChest <id>` loads the held item or collects; `Message`; other tile actions), `IndoorItems` placed when first built, `CollisionMap` (`X` blocks, others open; rows top-down) | `valley_sim::building_features` |
| Monster eradication goals: `Data/MonsterSlayerQuests` (`DisplayName`, `Targets` monster names, `Count`, `RewardItemId`, `RewardItemPrice` (sold at `AdventureShop` once claimed; -1 never), `RewardDialogue`, `RewardDialogueFlag`, `RewardFlag` (default `Gil_<id>`), `RewardFlagAll`, `RewardMail`, `RewardMailAll` (next morning)); kills are counted per monster name; tile actions `Port.KillList` (the goals board) and `Port.GuildMaster` (claim one reward per use); listed in key order | `valley_sim::monster_goals` |
| Furniture: `Data/Furniture` fields name/type/sprite size/footprint/rotations/price/placement (-1 or 0 indoors, 1 outdoors, 2 both)/display name/sprite index/texture. Rotations (1, 2, 4): the action button turns the held piece, use places it (empty-handed use picks it up); side views swap the footprint (chairs keep 1x1) and, approx., sit after the front view in the sheet (width = side footprint width, height + the footprint swap), then the back view; the left view mirrors the side. Tables (`table`, `long table`) hold one object; fireplaces toggle and use the sprite after the last view when lit; lamps (`lamp`, `sconce`, `torch`) glow after dusk; homes dim at night (approx.) | `valley_sim::furniture` |
| Sitting: seat furniture (`chair`, `bench`, `couch`, `armchair`) faces its rotation; map seats from `Data/ChairTiles` (`<sheet image>/<col>/<row>` -> `<width>/<height>/<direction up|down|left|right|opposite>/<type>/...`, sheet names matched with or without a season prefix) on the `Buildings` layer; moving, using a tool or warping stands up | `valley_sim::seats` |
| Weddings: `Data/Weddings` `EventScript` (per spouse or `default`) played in `Town` on the wedding morning, tokens `[SpouseActor]`, `[SetupContextualWeddingAttendees]` (each attendee's `Setup` when their `Condition` passes and they're around, unless `IgnoreUnlockConditions`), `[ContextualWeddingCelebrations]` (their `Celebration` commands); `end wedding` returns home. Without the data a message stands in | `valley_sim::weddings` |
| Jukeboxes: songs heard are remembered; a mini jukebox (`209`/`Dev_MiniJukebox`) or a `Jukebox` tile offers Off, Random (changes hourly) and the heard tracks, named by `Data/JukeboxTracks` (`Name`, `Available`, `AlternativeTrackIds` folded into one entry); the pick plays in that location while a jukebox is there | `valley_sim::jukebox`, `valley_game::music` |
| Incoming calls: `Data/IncomingPhoneCalls` (`TriggerCondition`, `RingCondition`, `FromNpc`, `FromDisplayName`, `Dialogue`, `SimpleDialogueSplitBy`, `IgnoreBaseChance`, `MaxCalls` default 1); one call a day at most, base chance 15% (approx.), rings between 9:00 and 17:00 for two hours if a telephone is placed; answering it at the phone plays the dialogue | `valley_sim::phone_calls` |
| Fish tanks: `fishtank` furniture holds fish (category -4; capacity width + 1, approx.), which swim inside using `Data/AquariumFish` sprite indexes on `LooseSprites/AquariumFish` (24x24 cells), else their item icon | `valley_sim::furniture` |
| Mannequins: `Data/Mannequins` (`DisplayName`, `Texture`, `SheetIndex`) as `(M)<id>` items placed like furniture (indoors or out); clothing or boots held go on (swapping that slot), empty hands swap outfits with the farmer, using it undresses then picks it up. Cursed mannequins' wandering isn't implemented | `valley_sim::furniture` |
| Audio changes: `Data/AudioChanges` entries (`Id`, `FilePaths` absolute or relative to the content folder, `.wav`/`.ogg`; one picked at random) add or replace cues ahead of every other source, mods included | `valley_content::audio` |
| Friendship: holding an object and interacting gives it (once a day); loved +80, liked +45, neutral +20, disliked -20, hated -40, positive gifts x1.1/1.25/1.5 for silver/gold/iridium, birthdays x8; two a week except on birthdays and to a spouse; daily drift when not talked to: friends -2 (none once maxed), dating/engaged -8, spouse -20 | `valley_sim::world` |
| Villager social settings (`Data/Characters`): `CanSocialize` (query; no talking or gifts when it fails), `CanReceiveGifts`, `SocialTab` (`UnknownUntilMet` shows "???", `AlwaysShown`, `HiddenUntilMet`, `HiddenAlways`), `Calendar` (`AlwaysShown`, `HiddenUntilMet`, `HiddenAlways`), `DumpsterDiveFriendshipEffect` (default -25, children 0) and `DumpsterDiveEmote` for villagers within 7 tiles of a searched trash can, `ItemDeliveryQuests`, `PerfectionScore`, `SpouseGiftJealousy` and `SpouseGiftJealousyFriendshipChange` (default -30), `SpouseWantsChildren`, `FlowerDanceCanDance` (default: romanceable), `WinterStarParticipant` | `valley_sim::social` |
| Building skins: `Data/Buildings` `Skins` (`Id`, `Name`, `Description`, `Texture`, `Condition`, `BuildCost`, `BuildMaterials`, `BuildDays`, `ShowAsSeparateConstructionEntry`); separate entries appear in the carpenter list as `<kind>|<skin>` with their own cost; console `skin <building id> [skin]` changes a building's look | `valley_sim::building_features` |
| Festival catches: `Port/FestivalCatches` ([{ `Festival`, `Fish`, `Reward`, `Location`? }]) give a reward item per matching fish caught while that passive festival runs; default: a golden tag `(O)TroutDerbyTag` for each rainbow trout `(O)138` during `TroutDerby` | `valley_sim::passive_festivals` |
| Special items (game code in the original; original ids, `Dev_*` placeholders, or custom field `Port.Special`): Stardrop (`434`, eaten: +34 max energy, refilled), Horse Flute (`911`, calls your horse outdoors, kept), Golden Animal Cracker (`GoldenAnimalCracker`, fed to an animal whose data allows `CanEatGoldenCrackers`: double produce for good), Treasure Totem (`TreasureTotem`, 6-10 artifact spots within 4 tiles, approx.), Fairy Dust (`872`, a working machine finishes at once), big chests (`BigChest`, `BigStoneChest`: 70 slots). Mystery boxes and other items with `GeodeDrops` crack at the blacksmith | `valley_sim::special_items` |
| Spouse days out: on dry days a spouse follows `marriage_<season>_<day>`, `marriage_<day>` or `marriage_<weekday>` from their schedule data (first found), starting from the farmhouse; otherwise they stay home | `valley_sim::romance` |
| Spouse chores (approx.): at 10+ hearts, 40% of mornings the spouse does one of: water the farm's crops (not when raining), fill empty troughs from the silos, mend fences to full, fill the pet's bowl, or make breakfast (a random cooked dish); they mention it when you talk | `valley_sim::romance` |
| Forge menu: weapon + gem (ids 60/62/64/66/68/70, or `Port.Forge` = Emerald/Aquamarine/Ruby/Amethyst/Topaz/Jade) up to 3 forges for 10/15/20 shards; diamond (72) fills them randomly; prismatic shard (74, `Port.Forge: Prismatic`) gives weapons Artful/Bug Killer/Crusader/Haymaker/Vampiric and tools Swift/Efficient/Powerful/Shavings/Archaeologist/Generous/Bottomless/Reaching (iridium)/Auto-Hook/Master/Preserving for 20; dragon tooth (852, `DragonTooth`) a random innate enchantment for 10; same-type weapon = look for 10; ring + ring = combined ring (`880` if present) for 20; galaxy weapons (4/23/29) + 3 galaxy souls (896) = infinity (62/64/63). Unforge refunds half. Effect sizes approx. (ruby +10% damage, aquamarine +4.6% crit, jade +0.1 crit power, amethyst +0.1 knockback, topaz +1 defense, emerald +1 speed; bug killer x2 vs bugs, crusader x1.5 vs undead and no mummy revive, slime slayer x1.5 vs slimes, vampiric 9% heal on kill, swift +50% swing speed, shavings 25% wood per hit, preserving 50% bait kept) | `valley_sim::forge` |

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
