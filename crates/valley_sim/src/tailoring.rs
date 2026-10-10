//! Tailoring: combine a fabric item with a second item at a sewing machine
//! (`Data/TailoringRecipes`, public format subset). The sewing machine is a
//! big craftable with the `Port.SewingMachine` custom field, or any tile
//! with the `Tailoring` action.

use engine::Rng;
use serde::Deserialize;

use crate::item::Item;
use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct TailoringRecipe {
    pub id: String,
    pub first_item_tags: Vec<String>,
    pub second_item_tags: Vec<String>,
    pub spend_right_item: Option<bool>,
    pub crafted_item_id: Option<String>,
    pub crafted_item_ids: Option<Vec<String>>,
}

impl World {
    pub fn is_sewing_machine(&self, big_id: &str) -> bool {
        self.items.big(big_id).is_some_and(|b| b.data.custom_fields.as_ref().is_some_and(|f| f.contains_key("Port.SewingMachine")))
    }

    /// The recipe that fits this pair, if any.
    pub fn tailoring_recipe(&self, left: &Item, right: &Item) -> Option<&TailoringRecipe> {
        self.tailoring.iter().find(|r| {
            let first = r.first_item_tags.is_empty() || self.items.matches_tags(left, &r.first_item_tags);
            let second = r.second_item_tags.is_empty() || self.items.matches_tags(right, &r.second_item_tags);
            first && second && (r.crafted_item_id.is_some() || r.crafted_item_ids.as_ref().is_some_and(|v| !v.is_empty()))
        })
    }

    /// Sew inventory items `left` (fabric) and `right`; returns the result.
    pub fn tailor(&mut self, left: usize, right: usize) -> Result<Item, String> {
        if left == right {
            return Err("Pick two different items.".into());
        }
        let (Some(l), Some(r)) = (self.farmer.inventory.get(left).cloned(), self.farmer.inventory.get(right).cloned()) else {
            return Err("Pick two items.".into());
        };
        let recipe = self.tailoring_recipe(&l, &r).cloned().ok_or("Those don't make anything.")?;
        let id = match (&recipe.crafted_item_ids, &recipe.crafted_item_id) {
            (Some(ids), _) if !ids.is_empty() => {
                let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 808]));
                rng.pick(ids).cloned().unwrap_or_default()
            }
            (_, Some(id)) => id.clone(),
            _ => return Err("Those don't make anything.".into()),
        };
        let out = self.items.create(&id, 1).ok_or("That pattern is missing.")?;
        self.farmer.inventory.reduce(left, 1);
        if recipe.spend_right_item.unwrap_or(true) {
            self.farmer.inventory.reduce(right, 1);
        }
        self.give_item(out.clone());
        self.sound("crafting");
        Ok(out)
    }

    pub(crate) fn open_tailoring(&mut self) -> bool {
        self.emit(WorldEvent::OpenTailoring);
        true
    }
}
