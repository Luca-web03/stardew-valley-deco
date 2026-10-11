//! Farm automation (public behaviour; details approx.):
//!
//! - auto-grabber (`165`): a chest in an animal building that each morning
//!   gathers the produce lying on the floor and takes the milk/wool-type
//!   produce off the animals living there;
//! - auto-petter (`272`): animals living in its building are petted each
//!   morning;
//! - mini shipping bin (`248`): ships the held item like the farm's bin;
//! - workbench (`208`): crafting there also uses the chests next to it;
//! - sprinkler attachments: a pressure nozzle (`915`) widens the area by a
//!   tile each way; an enricher (`913`) holds fertilizer (use fertilizer on
//!   the sprinkler) and spreads it on unfertilized soil it waters.
//!
//! Each kind is also recognised by a `Port.<Kind>` custom field
//! (`Port.AutoGrabber`, `Port.AutoPetter`, `Port.MiniShippingBin`,
//! `Port.Workbench`, `Port.Nozzle`, `Port.Enricher`).

use crate::inventory::Inventory;
use crate::item::Item;
use crate::objects::{ObjKind, SprinklerShape};
use crate::terrain::TerrainFeature;
use crate::world::{World, WorldEvent};
use crate::TilePos;

const AUTO_GRABBERS: &[&str] = &["165", "Dev_AutoGrabber"];
const AUTO_PETTERS: &[&str] = &["272", "Dev_AutoPetter"];
const MINI_BINS: &[&str] = &["248", "Dev_MiniBin"];
const WORKBENCHES: &[&str] = &["208", "Dev_Workbench"];
const NOZZLES: &[&str] = &["915", "Dev_Nozzle"];
const ENRICHERS: &[&str] = &["913", "Dev_Enricher"];

/// A sprinkler's area with its attachment.
pub fn sprinkler_area(shape: SprinklerShape, nozzle: bool) -> SprinklerShape {
    if !nozzle {
        return shape;
    }
    match shape {
        SprinklerShape::Cross => SprinklerShape::Square1,
        SprinklerShape::Square1 => SprinklerShape::Square2,
        _ => SprinklerShape::Square3,
    }
}

impl World {
    fn big_is(&self, id: &str, list: &[&str], field: &str) -> bool {
        list.contains(&id) || self.items.big(id).and_then(|b| b.data.custom_fields.as_ref()).is_some_and(|f| f.contains_key(field))
    }

    fn object_is(&self, id: &str, list: &[&str], field: &str) -> bool {
        list.contains(&id) || self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).is_some_and(|f| f.contains_key(field))
    }

    pub fn is_auto_grabber(&self, big_id: &str) -> bool {
        self.big_is(big_id, AUTO_GRABBERS, "Port.AutoGrabber")
    }

    pub fn is_auto_petter(&self, big_id: &str) -> bool {
        self.big_is(big_id, AUTO_PETTERS, "Port.AutoPetter")
    }

    pub fn is_mini_bin(&self, big_id: &str) -> bool {
        self.big_is(big_id, MINI_BINS, "Port.MiniShippingBin")
    }

    pub fn is_workbench(&self, big_id: &str) -> bool {
        self.big_is(big_id, WORKBENCHES, "Port.Workbench")
    }

    pub fn is_nozzle(&self, id: &str) -> bool {
        self.object_is(id, NOZZLES, "Port.Nozzle")
    }

    pub fn is_enricher(&self, id: &str) -> bool {
        self.object_is(id, ENRICHERS, "Port.Enricher")
    }

    /// Morning: auto-petters pet, auto-grabbers gather.
    pub(crate) fn automation_morning(&mut self) {
        let mut petted_homes = Vec::new();
        let mut grabbers: Vec<(String, TilePos)> = Vec::new();
        for (name, loc) in &self.locations {
            for (p, o) in &loc.state.objects {
                if o.big && self.is_auto_petter(&o.item_id) {
                    petted_homes.push(name.clone());
                }
                if o.big && self.is_auto_grabber(&o.item_id) && matches!(o.kind, ObjKind::Chest(_)) {
                    grabbers.push((name.clone(), *p));
                }
            }
        }
        for a in self.animals.iter_mut().filter(|a| petted_homes.contains(&a.home)) {
            a.petted_today = true;
        }
        for (name, at) in grabbers {
            // Produce lying on the floor.
            let floor: Vec<(TilePos, Item)> = self.locations[&name]
                .state
                .objects
                .iter()
                .filter(|(_, o)| matches!(o.kind, ObjKind::Forage))
                .map(|(p, o)| (*p, Item::object(&o.item_id, 1)))
                .collect();
            for (p, item) in floor {
                if self.grabber_store(&name, at, item) {
                    self.locations.get_mut(&name).unwrap().state.objects.remove(&p);
                }
            }
            // Produce waiting on the animals (milk, wool...).
            for i in 0..self.animals.len() {
                if self.animals[i].home != name {
                    continue;
                }
                let Some(item) = self.animals[i].produce.clone() else { continue };
                if self.grabber_store(&name, at, item) {
                    self.animals[i].produce = None;
                }
            }
        }
    }

    fn grabber_store(&mut self, loc: &str, at: TilePos, item: Item) -> bool {
        let Some(ObjKind::Chest(inv)) = self.locations.get_mut(loc).and_then(|l| l.state.objects.get_mut(&at)).map(|o| &mut o.kind) else { return false };
        if !inv.can_fit(&item) {
            return false;
        }
        inv.add(item);
        true
    }

    /// Action on a mini shipping bin or workbench.
    pub(crate) fn use_automation(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).filter(|o| o.big).cloned() else { return false };
        if self.is_mini_bin(&o.item_id) {
            if self.farmer.current_item().is_none() {
                self.hud("Hold an item to ship it.");
                return true;
            }
            return self.ship_current();
        }
        if self.is_workbench(&o.item_id) {
            self.crafting_station = Some((self.current.clone(), t));
            self.emit(WorldEvent::OpenCrafting);
            self.sound("bigSelect");
            return true;
        }
        false
    }

    /// Chests next to the workbench the farmer is crafting at.
    pub fn crafting_chests(&self) -> Vec<TilePos> {
        let Some((loc, at)) = &self.crafting_station else { return Vec::new() };
        if *loc != self.current || self.farmer.tile().manhattan(*at) > 3 {
            return Vec::new();
        }
        let l = self.location();
        let mut v = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                let p = TilePos::new(at.x + dx, at.y + dy);
                if matches!(l.object(p).map(|o| &o.kind), Some(ObjKind::Chest(_))) && (dx, dy) != (0, 0) {
                    v.push(p);
                }
            }
        }
        v
    }

    /// Can this recipe be crafted from the backpack (and workbench chests)?
    pub fn can_craft_here(&self, name: &str) -> bool {
        let Some(r) = self.recipes.get(name) else { return false };
        let chests = self.crafting_chests();
        let l = self.location();
        let mut invs: Vec<&Inventory> = vec![&self.farmer.inventory];
        for p in &chests {
            if let Some(ObjKind::Chest(inv)) = l.object(*p).map(|o| &o.kind) {
                invs.push(inv);
            }
        }
        r.can_craft_from(&self.items, &invs)
    }

    /// Consume a recipe's ingredients from the backpack, then the workbench
    /// chests. Returns the output.
    pub(crate) fn craft_with_station(&mut self, name: &str) -> Option<Item> {
        let r = self.recipes.get(name)?.clone();
        let chests = self.crafting_chests();
        let items = self.items.clone();
        let current = self.current.clone();
        let farmer = &mut self.farmer;
        let loc = self.locations.get_mut(&current)?;
        let mut invs: Vec<&mut Inventory> = vec![&mut farmer.inventory];
        for (p, o) in loc.state.objects.iter_mut() {
            if chests.contains(p) {
                if let ObjKind::Chest(inv) = &mut o.kind {
                    invs.push(inv);
                }
            }
        }
        r.craft_from(&items, &mut invs)
    }

    /// Use a nozzle, enricher or fertilizer on a sprinkler.
    pub(crate) fn attach_to_sprinkler(&mut self, t: TilePos) -> bool {
        let Some(item) = self.farmer.current_item().cloned() else { return false };
        let Some(id) = item.object_id().map(String::from) else { return false };
        if !matches!(self.location().object(t).map(|o| &o.kind), Some(ObjKind::Sprinkler { .. })) {
            return false;
        }
        let is_attachment = self.is_nozzle(&id) || self.is_enricher(&id);
        let is_fert = crate::machines::fertilizer_effect(&self.items, &id).is_some();
        let has_enricher = match self.location().object(t).map(|o| &o.kind) {
            Some(ObjKind::Sprinkler { attachment: Some(a), .. }) => self.is_enricher(a),
            _ => false,
        };
        let slot = self.farmer.current_slot;
        if is_attachment {
            let old = match self.location_mut().state.objects.get_mut(&t).map(|o| &mut o.kind) {
                Some(ObjKind::Sprinkler { attachment, .. }) => attachment.replace(id.clone()),
                _ => None,
            };
            self.farmer.inventory.reduce(slot, 1);
            if let Some(old) = old {
                self.give_item(Item::object(&old, 1));
            }
            self.sound("axe");
            return true;
        }
        if is_fert && has_enricher {
            let stored = match self.location_mut().state.objects.get_mut(&t).map(|o| &mut o.kind) {
                Some(ObjKind::Sprinkler { fertilizer, .. }) => fertilizer.take(),
                _ => None,
            };
            let mut stack = item.clone();
            if let Some(s) = stored {
                if s.can_stack_with(&stack) {
                    stack.set_stack(stack.stack() + s.stack());
                } else {
                    self.give_item(s);
                }
            }
            if let Some(ObjKind::Sprinkler { fertilizer, .. }) = self.location_mut().state.objects.get_mut(&t).map(|o| &mut o.kind) {
                *fertilizer = Some(stack);
            }
            self.farmer.inventory.take(slot);
            self.sound("Ship");
            return true;
        }
        false
    }

    /// Morning: enrichers spread their fertilizer on unfertilized soil.
    pub(crate) fn enrich_soil(&mut self) {
        let names: Vec<String> = self.locations.keys().cloned().collect();
        for name in names {
            let sprinklers: Vec<(TilePos, SprinklerShape)> = self.locations[&name]
                .state
                .objects
                .iter()
                .filter_map(|(p, o)| match &o.kind {
                    ObjKind::Sprinkler { radius, attachment: Some(a), fertilizer: Some(_) } if self.is_enricher(a) => Some((*p, *radius)),
                    _ => None,
                })
                .collect();
            for (p, shape) in sprinklers {
                let loc = self.locations.get_mut(&name).unwrap();
                let Some(ObjKind::Sprinkler { fertilizer, .. }) = loc.state.objects.get_mut(&p).map(|o| &mut o.kind) else { continue };
                let Some(mut fert) = fertilizer.take() else { continue };
                let fid = fert.object_id().unwrap_or("").to_string();
                for t in shape.tiles(p) {
                    if fert.stack() <= 0 {
                        break;
                    }
                    if let Some(TerrainFeature::HoeDirt(d)) = loc.state.terrain.get_mut(&t) {
                        if d.fertilizer.is_none() {
                            d.fertilizer = Some(fid.clone());
                            fert.set_stack(fert.stack() - 1);
                        }
                    }
                }
                if fert.stack() > 0 {
                    if let Some(ObjKind::Sprinkler { fertilizer, .. }) = loc.state.objects.get_mut(&p).map(|o| &mut o.kind) {
                        *fertilizer = Some(fert);
                    }
                }
            }
        }
    }
}
