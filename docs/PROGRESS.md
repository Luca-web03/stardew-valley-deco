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
- [x] Community bundles (Data/Bundles) and museum donations (Data/MuseumRewards)
- [x] Options: zoom, music/sound volume; snow/wind particles, night light pools
- [x] Audio: XACT wave banks (PCM 8/16-bit, MS-ADPCM), best-effort sound bank
      cue mapping + `Audio/cues.json` override, loose `.wav`/`.ogg`, software
      mixer (pitch, volume, looping crossfaded music), cpal (Windows/macOS) /
      runtime-loaded ALSA (Linux) output with silent fallback, original
      synthesized placeholder cues and music loops, music selection by
      location/season/weather/time

## Not yet
- [ ] Festivals
- [ ] Marriage, heart events beyond the generic event system
- [ ] Real-content farmer sprite composition (currently placeholder farmer)
- [ ] Real tree / building sprite layouts (placeholder art used)
- [ ] Key rebinding, gamepad
- [ ] Audio follow-ups: xWMA decoding, sound bank volume/pitch/categories,
      footsteps, ambient sounds
- [ ] Multiplayer
