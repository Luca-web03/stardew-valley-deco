//! Fruit trees (Data/FruitTrees), planting wild tree seeds (Data/WildTrees),
//! tappers and artifact spots (Data/Locations ArtifactSpots).

use engine::Rng;

use crate::collision::tile_passable;
use crate::item::Item;
use crate::objects::{ObjKind, PlacedObject};
use crate::terrain::{FruitTree, TerrainFeature, Tree};
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Well-known tapper ids (first available wins), placeholder last.
const TAPPERS: &[&str] = &["105", "264", "Dev_Tapper"];
/// Artifact spot object ids.
const ARTIFACT_SPOT: &[&str] = &["590", "Dev_ArtifactSpot"];

impl World {
    pub fn is_tapper(&self, item: &Item) -> bool {
        item.big_id().is_some_and(|id| TAPPERS.contains(&id))
    }

    /// Plant a fruit sapling or wild tree seed held in the current slot.
    pub(crate) fn plant_tree(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(id) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else { return false };
        let is_fruit = self.fruit_trees.contains_key(&id);
        let wild = self.wild_trees.iter().find(|(_, d)| d.seed_plantable && d.seed_item_id.as_deref().map(|s| crate::item::split_qualified(s).1) == Some(id.as_str())).map(|(k, _)| k.clone());
        if !is_fruit && wild.is_none() {
            return false;
        }
        let loc = self.location();
        if !loc.is_clear(t) || !tile_passable(loc, t) || self.farmer.tile() == t {
            self.hud("Can't plant that here.");
            return true;
        }
        if is_fruit && !(loc.is_farm || !loc.outdoors) {
            self.hud("Fruit trees can only be planted on the farm.");
            return true;
        }
        let feature = if is_fruit { TerrainFeature::FruitTree(FruitTree::new(&id)) } else { TerrainFeature::Tree(Tree::new(&wild.unwrap(), 0)) };
        self.location_mut().state.terrain.insert(t, feature);
        self.farmer.inventory.reduce(slot, 1);
        self.sound("dirtyHit");
        true
    }

    /// Shake a fruit tree / collect its fruit.
    pub(crate) fn shake_fruit_tree(&mut self, t: TilePos) -> bool {
        let Some(TerrainFeature::FruitTree(ft)) = self.location().terrain(t).cloned() else { return false };
        if ft.fruit.is_empty() {
            if let Some(TerrainFeature::FruitTree(f)) = self.location_mut().terrain_mut(t) {
                f.shake = 12;
            }
            self.sound("leafrustle");
            return true;
        }
        let q = ft.fruit_quality();
        for id in &ft.fruit {
            self.spawn_drop(Item::object_q(id, 1, q), t.world_center());
        }
        if let Some(TerrainFeature::FruitTree(f)) = self.location_mut().terrain_mut(t) {
            f.fruit.clear();
            f.shake = 12;
        }
        self.sound("leafrustle");
        true
    }

    /// Place a tapper on a grown tree.
    pub(crate) fn place_tapper(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return false };
        if !self.is_tapper(&item) {
            return false;
        }
        let Some(TerrainFeature::Tree(tree)) = self.location().terrain(t).cloned() else {
            self.hud("Tappers go on grown trees.");
            return true;
        };
        if !tree.mature() || tree.stump || self.location().object(t).is_some() {
            self.hud("Tappers go on grown trees.");
            return true;
        }
        let days = self.wild_trees.get(&tree.kind).and_then(|d| d.tap_items.first()).map(|ti| ti.days_until_ready.max(1)).unwrap_or(7);
        let id = item.big_id().unwrap().to_string();
        self.location_mut().state.objects.insert(t, PlacedObject::new(&id, true, ObjKind::Tapper { output: None, days_left: days }));
        if let Some(TerrainFeature::Tree(tr)) = self.location_mut().terrain_mut(t) {
            tr.tapped = true;
        }
        self.farmer.inventory.reduce(slot, 1);
        self.sound("axe");
        true
    }

    /// Collect from a tapper.
    pub(crate) fn collect_tapper(&mut self, t: TilePos) -> bool {
        let Some(PlacedObject { kind: ObjKind::Tapper { output, .. }, .. }) = self.location().object(t).cloned() else { return false };
        let Some(item) = output else { return true };
        if !self.farmer.inventory.can_fit(&item) {
            self.hud("Inventory full");
            return true;
        }
        self.farmer.inventory.add(item.clone());
        let kind = match self.location().terrain(t) {
            Some(TerrainFeature::Tree(tr)) => tr.kind.clone(),
            _ => String::new(),
        };
        let days = self.wild_trees.get(&kind).and_then(|d| d.tap_items.first()).map(|ti| ti.days_until_ready.max(1)).unwrap_or(7);
        if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
            o.kind = ObjKind::Tapper { output: None, days_left: days };
        }
        self.sound("coin");
        self.emit(WorldEvent::Pickup(item));
        true
    }

    /// Dig an artifact spot with the hoe.
    pub fn dig_artifact_spot(&mut self, t: TilePos) -> bool {
        let Some(PlacedObject { kind: ObjKind::ArtifactSpot, .. }) = self.location().object(t) else { return false };
        self.location_mut().state.objects.remove(&t);
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, t.x as i64, t.y as i64, 3]));
        // The location's drops and the shared Default ones, by precedence.
        let mut drops = self.location_data_for(&self.current).map(|d| d.artifact_spots.clone()).unwrap_or_default();
        if self.current != "Default" {
            if let Some(d) = self.location_data.get("Default") {
                drops.extend(d.artifact_spots.iter().cloned());
            }
        }
        drops.sort_by_key(|d| d.precedence);
        let slot = self.farmer.current_slot;
        // Archaeologist doubles each find's chance; Generous may dig up a
        // second copy (approx.).
        let boost = if self.tool_has(slot, "Archaeologist") { 2.0 } else { 1.0 };
        let generous = self.tool_has(slot, "Generous");
        let mut got = false;
        for d in drops {
            if !self.check_query(d.condition.as_deref(), None, None, &mut rng.clone()) {
                continue;
            }
            if rng.chance((d.chance.unwrap_or(1.0) * boost).min(1.0)) {
                let n = if d.max_stack > d.min_stack { rng.range(d.min_stack.max(1), d.max_stack + 1) } else { d.min_stack.max(1) };
                let id = match &d.random_item_id {
                    Some(ids) if !ids.is_empty() => rng.pick(ids).cloned().unwrap_or_default(),
                    _ => d.item_id.clone(),
                };
                if let Some(item) = self.items.create(&id, n) {
                    if generous && d.apply_generous_enchantment && rng.chance(0.5) {
                        self.spawn_drop(item.clone(), t.world_center());
                    }
                    self.spawn_drop(item, t.world_center());
                    got = true;
                    if !d.continue_on_drop {
                        break;
                    }
                }
            }
        }
        if !got {
            self.spawn_drop(Item::object(&self.mine_items.stone.clone(), 1), t.world_center());
        }
        self.maybe_drop_note(&mut rng, t.world_center());
        self.sound("hoeHit");
        true
    }

    /// Overnight: fruit growth, tappers, artifact spot spawns.
    pub(crate) fn orchard_overnight(&mut self) {
        let season = self.clock.season;
        let fruit_data = self.fruit_trees.clone();
        let wild = self.wild_trees.clone();
        let items = self.items.clone();
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 919]));
        for loc in self.locations.values_mut() {
            let greenhouse = !loc.outdoors;
            let tree_kinds: std::collections::BTreeMap<TilePos, String> =
                loc.state.terrain.iter().filter_map(|(p, f)| match f { TerrainFeature::Tree(t) => Some((*p, t.kind.clone())), _ => None }).collect();
            for f in loc.state.terrain.values_mut() {
                let TerrainFeature::FruitTree(ft) = f else { continue };
                if !ft.mature() {
                    ft.days_until_mature -= 1;
                    continue;
                }
                ft.age_after_mature += 1;
                let Some(d) = fruit_data.get(&ft.kind) else { continue };
                let in_season = greenhouse || d.seasons.iter().any(|s| crate::time::Season::parse(s) == Some(season));
                if in_season && ft.fruit.len() < 3 {
                    let opts: Vec<_> = d.fruit.iter().filter(|fr| fr.season.as_deref().map_or(true, |s| crate::time::Season::parse(s) == Some(season) || greenhouse)).collect();
                    if let Some(fr) = rng.pick(&opts) {
                        if rng.chance(fr.chance.unwrap_or(1.0)) {
                            ft.fruit.push(crate::item::split_qualified(&fr.item_id).1.to_string());
                        }
                    }
                }
            }
            for (p, o) in loc.state.objects.iter_mut() {
                let ObjKind::Tapper { output, days_left } = &mut o.kind else { continue };
                if output.is_some() {
                    continue;
                }
                *days_left -= 1;
                if *days_left <= 0 {
                    let kind = tree_kinds.get(p).cloned().unwrap_or_default();
                    if let Some(ti) = wild.get(&kind).and_then(|d| d.tap_items.first()) {
                        *output = items.create(&ti.item_id, ti.min_stack.max(1));
                    }
                }
            }
            // Artifact spots appear on diggable outdoor tiles.
            if loc.outdoors && !loc.is_farm {
                let spot = ARTIFACT_SPOT.iter().find(|id| items.objects.contains_key(**id)).map(|s| s.to_string());
                let existing = loc.state.objects.values().filter(|o| matches!(o.kind, ObjKind::ArtifactSpot)).count();
                if let Some(spot) = spot {
                    if existing < 3 && rng.chance(0.5) {
                        for _ in 0..40 {
                            let t = TilePos::new(rng.range(0, loc.width()), rng.range(0, loc.height()));
                            if loc.is_clear(t) && loc.is_diggable(t) && tile_passable(loc, t) {
                                loc.state.objects.insert(t, PlacedObject::new(&spot, false, ObjKind::ArtifactSpot));
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
