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
      random item queries, daily stock limits, trade items, star tokens /
      Qi coins / Qi gems), traveling cart
- [x] Crafting, placeables, chests, data-driven machines (Data/Machines),
      furniture, casks, tailoring at the sewing machine
- [x] Cooking, food buffs
- [x] Fishing (cast, bite, minigame, size/quality/XP), crab pots, bait and
      tackle, treasure chests
- [x] Mines: levels, rocks/ores, ladders, elevator, bombs; monsters,
      weapons, rings, boots, combat, fainting
- [x] Weapon special moves (block, flurry, slam; approx. timings),
      slingshot with ammo and projectiles
- [x] Deep cavern below the mines (shafts, tougher floors), paid travel
      between areas, warp totems and obelisks, placeholder desert
- [x] Stables and horse riding, minecart networks, TV channels with the
      cooking show
- [x] Island-style progression (collectible counters, unlock actions,
      flag-cleared tiles) and a generated volcano dungeon with lava and a
      weapon forge; placeholder isle
- [x] Farm buildings, silos and hay, animals (feeding, petting, produce,
      friendship), farmhouse upgrades, kitchen fridge, fish ponds, harvest huts
- [x] NPCs: schedules, routing across locations, dialogue with questions,
      gifts, friendship, dating, marriage, spouse, children (approx. timings),
      divorce, jealousy
- [x] Special orders board (weekly offers, one active order; Data/SpecialOrders
      subset)
- [x] Mail, quests, help-wanted board, journal, bundles, museum,
      collections, achievements (engine-defined milestones), secret notes,
      perfection score (engine-defined)
- [x] Casino with original games (reels, high-low), Qi coins, prize shop
- [x] Farm pet (wanders, petting, water bowl, stays in on rainy days),
      trash cans (Data/GarbageCans)
- [x] Cutscene events (core command subset), festivals, passive festivals
      (multi-day map swaps)
- [x] Movie theater: seasonal movies, tickets, inviting villagers,
      reactions by movie tags (no concessions yet)
- [x] Skills, professions, level-up screen, mastery (engine-defined perks),
      books (skill XP, powers), trinkets (stat effects)
- [x] Co-op: host/join over TCP, farmhands with their own farmer, location,
      tools and menus (swapped into the shared world for their turn),
      render streaming (lz4 draw lists, textures, sounds) so clients need no
      content, farmhands saved with the host's game (no cabins or
      split-screen; the host's sleep ends the day)
- [x] Saves (JSON), title screen, character creation, load list
- [x] Options (zoom, volumes, key bindings), gamepad, console (F1),
      debug overlay (F3)
- [x] Audio: XACT wave banks (PCM 8/16-bit, MS-ADPCM), best-effort sound bank
      cue mapping + `Audio/cues.json` override, loose `.wav`/`.ogg`, software
      mixer, platform output with silent fallback, original synthesized
      placeholder cues and music, music by location/season/weather/time

## Not yet
- [ ] The original's island/volcano/desert specifics that are game code
      (golden walnut locations, parrots, volcano layouts and forge
      enchantments, desert festival); generic counterparts exist:
      collectibles + unlocks, flag-cleared tiles, a generated volcano with
      lava and a forge
- [ ] Default layer values for the real farmer sheets (the layered
      compositor exists and is configured in `valley_layouts.json`
      `farmer.layers`; the right rows/offsets for the original sheets have to
      be filled in against the real files)
- [ ] Audio follow-ups: xWMA decoding (convert such tracks to
      `Audio/<cue>.ogg`), sound bank volume/pitch/categories
