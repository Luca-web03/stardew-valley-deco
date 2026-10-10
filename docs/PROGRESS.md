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
- [x] Content mods: folder overlays with entry-level JSON merging

## Gameplay
- [x] Time, calendar, weather (approx. chances), storms with lightning,
      night darkening and light pools, rain / snow / wind visuals
- [x] Farmer movement/collision, warps, doors, bed, passing out, footsteps,
      appearance customization, clothing (hats, shirts, pants)
- [x] Tools: hoe, can, axe, pickaxe, scythe, rod, milk pail, shears;
      stamina; upgrades at the forge; charged area swings
- [x] Crops: phases, watering, regrowth, seasons, quality formula,
      fertilizer (quality / speed / retention), sprinklers, greenhouse
- [x] Trees (chop, stump, growth — approx.), tree seeds, tappers,
      fruit trees, debris, grass, forage, artifact spots
- [x] Items, inventory, toolbar, dropped items, shipping, shops (buy/sell,
      random item queries, daily stock limits), traveling cart
- [x] Crafting, placeables, chests, data-driven machines (Data/Machines),
      furniture
- [x] Cooking, food buffs
- [x] Fishing (cast, bite, minigame, size/quality/XP), crab pots
- [x] Mines: levels, rocks/ores, ladders, elevator, bombs; monsters,
      weapons, rings, boots, combat, fainting
- [x] Weapon special moves (block, flurry, slam; approx. timings),
      slingshot with ammo and projectiles
- [x] Farm buildings, silos and hay, animals (feeding, petting, produce,
      friendship), farmhouse upgrades, kitchen fridge, fish ponds
- [x] NPCs: schedules, routing across locations, dialogue with questions,
      gifts, friendship, dating, marriage, spouse, children (approx. timings)
- [x] Special orders board (weekly offers, one active order; Data/SpecialOrders
      subset)
- [x] Mail, quests, help-wanted board, journal, bundles, museum,
      collections, achievements (engine-defined milestones)
- [x] Farm pet (wanders, petting, water bowl, stays in on rainy days),
      trash cans (Data/GarbageCans)
- [x] Cutscene events (core command subset), festivals
- [x] Skills, professions, level-up screen
- [x] Saves (JSON), title screen, character creation, load list
- [x] Options (zoom, volumes, key bindings), gamepad, console (F1),
      debug overlay (F3)
- [x] Audio: XACT wave banks (PCM 8/16-bit, MS-ADPCM), best-effort sound bank
      cue mapping + `Audio/cues.json` override, loose `.wav`/`.ogg`, software
      mixer, platform output with silent fallback, original synthesized
      placeholder cues and music, music by location/season/weather/time

## Not yet
- [x] Deep cavern below the mines (shafts, tougher floors), paid travel
      between areas, warp totems and obelisks, placeholder desert
- [ ] Area-specific mechanics beyond that (island, volcano, desert shops
      and events)
- [ ] Real-content farmer sprite composition (generated placeholder farmer
      used; any simple sheet can be configured in `valley_layouts.json`)
- [ ] Audio follow-ups: xWMA decoding (convert such tracks to
      `Audio/<cue>.ogg`), sound bank volume/pitch/categories
- [ ] Multiplayer
