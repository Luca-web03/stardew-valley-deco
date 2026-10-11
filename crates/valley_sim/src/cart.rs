//! The traveling cart: parks on Fridays and Sundays at the map tile named by
//! the `TravelingCartTile` map property and sells the `Traveler` shop's
//! stock (or a generated random stock when that shop is missing).

use crate::shop::{self, ShopEntry, ShopStock};
use crate::gsq::QueryCtx;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Cart footprint in tiles (top-left at the property tile).
pub const CART_W: i32 = 3;
pub const CART_H: i32 = 2;
/// Hours the cart is open (military time).
pub const CART_OPEN: i32 = 600;
pub const CART_CLOSE: i32 = 2000;
const FALLBACK_ITEMS: usize = 8;

impl World {
    /// Does the cart visit today?
    pub fn cart_day(&self) -> bool {
        matches!(self.clock.weekday(), "Fri" | "Sun")
    }

    /// The location and top-left tile where the cart parks.
    pub fn cart_spot(&self) -> Option<(String, TilePos)> {
        self.locations.iter().find_map(|(name, loc)| loc.map_prop_tile("TravelingCartTile").map(|t| (name.clone(), t)))
    }

    /// Is the cart parked here right now?
    pub fn cart_here(&self, location: &str) -> Option<TilePos> {
        if !self.cart_day() {
            return None;
        }
        self.cart_spot().filter(|(l, _)| l == location).map(|(_, t)| t)
    }

    fn cart_footprint(t: TilePos) -> impl Iterator<Item = TilePos> {
        (0..CART_W).flat_map(move |dx| (0..CART_H).map(move |dy| TilePos::new(t.x + dx, t.y + dy)))
    }

    /// Morning: park or remove the cart (its bottom row blocks walking).
    pub(crate) fn cart_morning(&mut self) {
        let Some((loc_name, t)) = self.cart_spot() else { return };
        let here = self.cart_day();
        if let Some(loc) = self.locations.get_mut(&loc_name) {
            for p in Self::cart_footprint(t).filter(|p| p.y == t.y + CART_H - 1) {
                if here {
                    loc.blocked.insert(p);
                } else {
                    loc.blocked.remove(&p);
                }
            }
        }
    }

    /// Action on the cart opens its shop.
    pub(crate) fn cart_action(&mut self, target: TilePos) -> bool {
        let Some(t) = self.cart_here(&self.current) else { return false };
        if !Self::cart_footprint(t).any(|p| p == target) {
            return false;
        }
        if !(CART_OPEN..CART_CLOSE).contains(&self.clock.time) {
            self.emit(WorldEvent::Message("The cart is closed for the night.".into()));
            return true;
        }
        let stock = self.cart_stock();
        self.emit(WorldEvent::OpenShop(stock));
        true
    }

    /// Today's cart stock; items bought earlier today are gone.
    pub fn cart_stock(&mut self) -> ShopStock {
        let mut rng = self.day_rng(17);
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng, world: Some(self) };
        let mut stock = match self.shops.get("Traveler") {
            Some(data) => shop::build_stock("Traveler", data, &self.items, &mut ctx),
            None => {
                let (ids, _) = shop::expand_item_query("RANDOM_ITEMS (O) @requirePrice @isRandomSale", &self.items, &mut ctx);
                let entries = ids
                    .into_iter()
                    .take(FALLBACK_ITEMS)
                    .filter_map(|id| self.items.create(&id, 1))
                    .map(|item| {
                        let price = shop::random_sale_price(&self.items, &item, &mut ctx);
                        ShopEntry { item, price, stock: 1, limit: 1, currency: 0, trade: None }
                    })
                    .collect();
                ShopStock { id: "Traveler".into(), owner: None, entries }
            }
        };
        self.apply_shop_sold(&mut stock);
        stock
    }
}
