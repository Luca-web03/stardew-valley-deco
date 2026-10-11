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
      fruit trees, debris, grass, forage, artifact spots, resource clumps
      (large stumps, logs, boulders, meteorites; approx. health)
- [x] Giant crops (Data/GiantCrops), crows and scarecrows, night events
      (fairy, witch, meteorite, owl, capsule, earthquake; approx. chances),
      rain/weather totems, green rain (one summer day a year: mossy weeds,
      tree growth spurt; approx. densities)
- [x] Items, inventory, toolbar, dropped items, shipping, shops (buy/sell,
      random item queries, daily stock limits, trade items, star tokens /
      Qi coins / Qi gems), traveling cart
- [x] Wallpaper and flooring for the rooms of your home
- [x] Swimming (swimsuit, pool entrances) and the spa; telephone calls
- [x] Bushes (berry seasons, tea bushes, cutting), `Paths` layer spawns
- [x] Automation: auto-grabber, auto-petter, mini shipping bin, workbench
      crafting from nearby chests, pressure nozzle, enricher
- [x] Fences (wear, falling over, removal tools), gates, floors and paths
      (joined sprites, speed bonus on the farm)
- [x] Crafting, placeables, chests, data-driven machines (Data/Machines,
      including day-update / put-down / output-collected triggers and the
      seed maker, flavored artisan goods, bee houses with flower honey),
      furniture, casks, tailoring at the sewing machine
- [x] Cooking, food buffs
- [x] Fishing (cast, bite, minigame, size/quality/XP), crab pots, bait and
      tackle, treasure chests
- [x] Mines: levels, rocks/ores, ladders, elevator, bombs; monsters,
      weapons, rings, boots, combat, fainting
- [x] Monster species behaviours by name (approx.): rock-disguised shelled
      crabs, burrowers, bone/spell/fireball throwers, healing shamans,
      reviving mummies, exploding hot heads, splitting big slimes, grubs
      turning into flies, quick dust sprites
- [x] Weapon special moves (block, flurry, slam; approx. timings),
      slingshot with ammo and projectiles; weapons cut grass and weeds
- [x] Forge: gem forges, tool and weapon enchantments with their effects,
      innate enchantments, weapon looks, combined rings, galaxy souls,
      unforging (approx. effect sizes)
- [x] Deep cavern below the mines (shafts, tougher floors), paid travel
      between areas, warp totems and obelisks, placeholder desert
- [x] Stables and horse riding, minecart networks, TV channels with the
      cooking show
- [x] Island-style progression (collectible counters, unlock actions,
      flag-cleared tiles) and a generated volcano dungeon with lava and a
      weapon forge; placeholder isle
- [x] Farm buildings, silos and hay, animals (feeding, petting, produce,
      friendship), farmhouse upgrades, kitchen fridge, fish ponds, harvest huts,
      slime hutches (troughs, slime balls, incubators), cabins for farmhands
- [x] NPCs: schedules (documented key fallbacks, `rain2`, `NOT friendship`,
      arrival times), routing across locations, dialogue with questions,
      chance/world-state/answer/once-only commands, `%` tokens, item gifts,
      rainy-day lines, gifts, friendship, dating, marriage, spouse, children
      (approx. timings), divorce, jealousy
- [x] Special orders board (weekly offers, one active order; Data/SpecialOrders
      subset)
- [x] Grandpa's evaluation (year 3, candles, diamond re-evaluation),
      Joja membership and community development projects
- [x] Mail, quests, help-wanted board, journal, bundles (standard or
      remixed from Data/RandomBundles), museum,
      collections, achievements (original ids/names from Data/Achievements,
      or engine-defined milestones), secret notes,
      perfection score (engine-defined)
- [x] Casino with original games (reels, high-low), Qi coins, prize shop
- [x] Arcade cabinets with two original minigames (Dust Ranger arena
      shooter, Rail Runner cart game), also hosted as standalone modules
- [x] Farm pet (wanders, petting, water bowl, stays in on rainy days),
      trash cans (Data/GarbageCans)
- [x] Cutscene events: questions and forks, quick questions, actor
      animation/frames/hops/shakes, speech above heads, temporary actors,
      props and animated sprites, temporary map edits, camera pans, ambient
      light, rewards and state commands, `end` variants; emote sheet;
      HUD hidden during cutscenes (code-only commands are skipped)
- [x] Festivals with activities: egg hunt, flower dance partners, luau
      soup, grange display judging for star tokens, ice fishing contest
      (simulated), winter feast secret gift exchange (approx. scoring);
      passive festivals (multi-day map swaps)
- [x] Movie theater: seasonal movies, tickets, inviting villagers,
      reactions by movie tags, concessions with guest tastes
- [x] Farm cave (mushrooms or fruit bats), Qi challenge board with gem
      rewards, dish of the day
- [x] Skills, professions, level-up screen, mastery (engine-defined perks),
      books (skill XP, powers), trinkets (stat effects)
- [x] Co-op: host/join over TCP, farmhands with their own farmer, location,
      tools and menus (swapped into the shared world for their turn),
      render streaming (lz4 draw lists, textures, sounds) so clients need no
      content, farmhands saved with the host's game and living in cabins
      (no split-screen; the host's sleep ends the day)
- [x] Farm types (Standard, Riverland, Forest, Hill-top, Wilderness, Four
      Corners, Beach, Meadowlands, plus Data/AdditionalFarms) with their maps,
      `Farm_<type>` location data and approx. perks (quarry rocks, night
      monsters, starter coop)
- [x] Saves (JSON), title screen, character creation, load list
- [x] World map (M): `Data/WorldMap` regions, area overlays, tooltips and
      player markers
- [x] Options (zoom, volumes, key bindings), gamepad, console (F1),
      debug overlay (F3)
- [x] Audio: XACT wave banks (PCM 8/16-bit, MS-ADPCM), best-effort sound bank
      cue mapping + `Audio/cues.json` override, loose `.wav`/`.ogg`, software
      mixer, platform output with silent fallback, original synthesized
      placeholder cues and music, music by location/season/weather/time

## Not yet
- [ ] The original's island/volcano/desert specifics that are game code
      (golden walnut locations, parrots, volcano layouts, desert
      festival); generic counterparts exist:
      collectibles + unlocks, flag-cleared tiles, a generated volcano with
      lava and a forge
- [ ] Default layer values for the real farmer sheets (the layered
      compositor exists and is configured in `valley_layouts.json`
      `farmer.layers`; the right rows/offsets for the original sheets have to
      be filled in against the real files)
- [ ] Audio follow-ups: xWMA decoding (convert such tracks to
      `Audio/<cue>.ogg`), sound bank volume/pitch/categories
