use valley_content::data::ShopData;

use crate::gsq::{self, QueryCtx};
use crate::item::{Item, ItemDb};

#[derive(Clone, Debug, PartialEq)]
pub struct ShopEntry {
    pub item: Item,
    pub price: i32,
    /// -1 = unlimited.
    pub stock: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopStock {
    pub id: String,
    pub owner: Option<String>,
    pub entries: Vec<ShopEntry>,
}

/// Default purchase price of an object is twice its sell price.
pub fn default_buy_price(db: &ItemDb, item: &Item) -> i32 {
    match item {
        Item::Object { id, .. } => db.object(id).map(|o| o.data.price * 2).unwrap_or(0),
        Item::Big { id, .. } => db.big(id).map(|o| o.data.price * 2).unwrap_or(0),
        Item::Boots { id } => db.boots.get(id).map(|b| b.price).unwrap_or(0),
        Item::Furniture { id } => db.furniture.get(id).map(|f| f.price).unwrap_or(0),
        Item::Tool(_) | Item::Weapon { .. } => 0,
    }
}

pub fn build_stock(id: &str, data: &ShopData, db: &ItemDb, ctx: &mut QueryCtx) -> ShopStock {
    let mut entries = Vec::new();
    for e in &data.items {
        if !gsq::check(e.condition.as_deref(), ctx) {
            continue;
        }
        let Some(item) = db.create(&e.item_id, 1) else { continue };
        let price = if e.price >= 0 { e.price } else { default_buy_price(db, &item) };
        entries.push(ShopEntry { item, price, stock: e.available_stock });
    }
    let owner = data.owners.first().map(|o| o.name.clone()).filter(|n| !n.is_empty() && n != "AnyOrNone" && n != "Any" && n != "None");
    ShopStock { id: id.to_string(), owner, entries }
}
