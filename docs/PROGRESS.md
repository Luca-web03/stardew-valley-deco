# Progress

Status of the clean-room reimplementation. "Approx." marks behaviour
rebuilt from public documentation where exact original values or formulas
were not available; these are isolated constants that can be tuned.

## Done

- [x] Engine core: module host, draw lists, software renderer, input, RNG, font
- [x] Content: PNG/JSON/TMX/TBIN/XNB(LZ4) readers, layered sources, string tokens
- [x] Placeholder pack (original art): farm, farmhouse, town, shop, 2 villagers
- [x] Time: 7s per 10 minutes, 6am–2am, 28-day seasons, years, weekdays
- [x] Farmer: movement (per-axis, run/walk), collision, warps, doors, bed
- [x] Tools: hoe, watering can (capacity, refill), axe, pickaxe, scythe (3-tile arc)
- [x] Stamina: 2 − 0.1 × skill level per swing; exhaustion
- [x] Crops: phases, watering, regrowth, seasons/withering, quality formula, harvest XP
- [x] Debris (weeds/twigs/stones), trees (chop, stump, growth — approx. values)
- [x] Items, inventory stacking (999), toolbar, dropped items with magnetism
- [x] Shipping bin, overnight sales, quality price multipliers
- [x] Day end: sleep/pass-out, money penalty, next-day energy (approx.)
- [x] Weather rolls (approx. chances), rain auto-watering, night darkening
- [x] NPCs: data loading, schedules (keys, GOTO), multi-location routing, A*
- [x] Dialogue: key priority, pages, emotions/portraits, @ and gender switch
- [x] Friendship: talk +20/day, gifts (tastes, weekly limit, birthday x8), decay
- [x] Shops from Data/Shops with game state queries; buy/sell
- [x] Menus: title, new game, load, inventory (items/skills/social/options), shop,
      dialogue, questions, day-end summary; autosave each night
- [x] Desktop window (macroquad) + headless scripted runner
- [x] Audio: XACT wave banks (PCM 8/16-bit, MS-ADPCM), best-effort sound bank
      cue mapping + `Audio/cues.json` override, loose `.wav`/`.ogg`, software
      mixer (pitch, volume, looping crossfaded music), cpal (Windows/macOS) /
      runtime-loaded ALSA (Linux) output with silent fallback, original
      synthesized placeholder cues and music loops, music selection by
      location/season/weather/time

## Next

- [ ] Crafting & placeable objects (chests, sprinklers, scarecrows, fences)
- [ ] Artisan machines (Data/Machines), fertilizer, fruit trees
- [ ] Forage and artifact spot spawning (Data/Locations)
- [ ] Fishing minigame (Data/Fish)
- [ ] Mines: levels, rocks/ores, monsters, combat, weapons, health
- [ ] Animals and farm buildings
- [ ] Cooking, recipes, buffs
- [ ] Mail (Data/mail), quests, help-wanted board
- [ ] Events/cutscenes script interpreter, festivals
- [ ] Audio follow-ups: xWMA decoding, XSB volume/pitch/categories, footsteps,
      ambient sounds, volume options in the menu
- [ ] Real-content farmer sprite composition, real tree/building layouts
- [ ] Bundles, museum, professions, marriage, multiplayer (out of scope for now)
