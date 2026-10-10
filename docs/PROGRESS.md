# Progress

Status of the clean-room reimplementation. "Approx." marks behaviour
rebuilt from public documentation where exact original values or formulas
were not available; such values are isolated constants that can be tuned.

## Engine & content
- [x] Engine core: module host (multi-game), draw lists, software renderer,
      input, RNG, bitmap font, console commands (`GameModule::command`)
- [x] Content: PNG / JSON / TMX / TBIN / XNB (uncompressed, LZ4) readers,
      layered sources (your content folder over the placeholder pack),
      tokenized strings
- [x] Original placeholder pack: farm, farmhouse, town, shop, mine entrance,
      generated mine levels, coop/barn interiors, two villagers, items,
      crops, fish, monsters, animals, machines, recipes, letters, quests,
      an intro event
- [x] Desktop window (macroquad) and headless scripted runner with screenshots

## Gameplay
- [x] Time, calendar, weather (approx. chances), night darkening, rain
- [x] Farmer movement/collision, warps, doors, bed, passing out
- [x] Tools: hoe, can, axe, pickaxe, scythe, rod, milk pail, shears;
      stamina; upgrades at the forge; charged area swings
- [x] Crops: phases, watering, regrowth, seasons, quality formula,
      fertilizer (quality / speed / retention), sprinklers
- [x] Debris, trees (chop, stump, growth — approx.), forage spawning
- [x] Items, inventory, toolbar, dropped items, shipping, shops (buy/sell)
- [x] Crafting, placeables, chests, data-driven machines (Data/Machines)
- [x] Cooking, food buffs
- [x] Fishing (cast, bite, minigame, size/quality/XP) from Data/Fish
- [x] Mines: levels, rocks/ores, ladders, elevator; monsters, weapons,
      combat, fainting
- [x] Farm buildings (construction, interiors) and animals (feeding,
      petting, produce, friendship)
- [x] NPCs: schedules, routing across locations, dialogue, gifts,
      friendship
- [x] Mail, quests, help-wanted board, journal
- [x] Cutscene events (core command subset)
- [x] Skills, professions, level-up screen
- [x] Saves (JSON), title screen, character creation, load list
- [x] Audio: being integrated (wave bank reader, mixer, placeholder synth)

## Not yet
- [ ] Community bundles, museum
- [ ] Festivals
- [ ] Marriage, heart events beyond the generic event system
- [ ] Real-content farmer sprite composition (currently placeholder farmer)
- [ ] Real tree / building sprite layouts (placeholder art used)
- [ ] Options menu (volume, zoom, key bindings), gamepad
- [ ] Snow / wind visuals, lighting from lamps
- [ ] Multiplayer
