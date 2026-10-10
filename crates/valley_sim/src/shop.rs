use valley_content::data::ShopData;

use crate::gsq::{self, QueryCtx};
use crate::item::{Item, ItemDb};

#[derive(Clone, Debug, PartialEq)]
pub struct ShopEntry {
    pub item: Item,
    pub price: i32,
    /// Remaining today; -1 = unlimited.
    pub stock: i32,
    /// Daily limit the entry started with; -1 = unlimited.
    pub limit: i32,
    /// Shop currency: 0 money, 1 star tokens, 2 Qi coins, 4 Qi gems.
    pub currency: i32,
    /// Items paid per purchase: (unqualified object id, amount).
    pub trade: Option<(String, i32)>,
}

/// Display name of a shop currency.
pub fn currency_name(c: i32) -> &'static str {
    match c {
        1 => "tokens",
        2 => "Qi coins",
        4 => "Qi gems",
        _ => "g",
    }
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
        Item::Clothing { slot, id } => db.clothing(*slot, id).map(|c| c.price).unwrap_or(0),
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
        let item_id = match &e.random_item_id {
            Some(ids) if !ids.is_empty() => ctx.rng.pick(ids).cloned().unwrap_or_default(),
            _ => e.item_id.clone(),
        };
        let (ids, random_sale) = expand_item_query(&item_id, db, ctx);
        let limit = e.max_items.filter(|n| *n >= 0).map(|n| n as usize).unwrap_or(usize::MAX);
        for qid in ids.into_iter().take(limit) {
            let Some(item) = db.create(&qid, 1) else { continue };
            let price = if e.price >= 0 {
                e.price
            } else if random_sale {
                random_sale_price(db, &item, ctx)
            } else {
                default_buy_price(db, &item)
            };
            let trade = e.trade_item_id.as_deref().map(|t| (crate::item::split_qualified(t).1.to_string(), e.trade_item_amount.unwrap_or(1).max(1)));
            // Trade-only entries cost nothing unless a price is given.
            let price = if trade.is_some() && e.price < 0 { 0 } else { price };
            entries.push(ShopEntry { item, price, stock: e.available_stock, limit: e.available_stock, currency: data.currency, trade });
        }
    }
    let owner = data.owners.first().map(|o| o.name.clone()).filter(|n| !n.is_empty() && n != "AnyOrNone" && n != "Any" && n != "None");
    ShopStock { id: id.to_string(), owner, entries }
}

/// Expand an item query into concrete item ids. Supports plain ids and
/// `RANDOM_ITEMS <type> [min id] [max id] [@requirePrice] [@isRandomSale]`
/// for objects. Returns the ids and whether random-sale pricing applies.
pub fn expand_item_query(query: &str, db: &ItemDb, ctx: &mut QueryCtx) -> (Vec<String>, bool) {
    let toks: Vec<&str> = query.split_whitespace().collect();
    if toks.first().copied() != Some("RANDOM_ITEMS") {
        return (vec![query.to_string()], false);
    }
    let kind = toks.get(1).copied().unwrap_or("(O)");
    if kind != "(O)" {
        return (vec![], false);
    }
    let flags: Vec<&str> = toks.iter().skip(2).filter(|t| t.starts_with('@')).copied().collect();
    let nums: Vec<i64> = toks.iter().skip(2).filter_map(|t| t.parse().ok()).collect();
    let (lo, hi) = (nums.first().copied(), nums.get(1).copied());
    let require_price = flags.contains(&"@requirePrice");
    let random_sale = flags.contains(&"@isRandomSale");
    let mut ids: Vec<String> = db
        .objects
        .iter()
        .filter(|(id, o)| {
            let in_range = match id.parse::<i64>() {
                Ok(n) => lo.map_or(true, |l| n >= l) && hi.map_or(true, |h| n <= h),
                Err(_) => lo.is_none() && hi.is_none(),
            };
            let priced = !require_price || o.data.price > 0;
            // Random sales skip junk and quest items (approx.).
            let saleable = !random_sale || !matches!(o.data.kind.as_str(), "Litter" | "Quest");
            in_range && priced && saleable
        })
        .map(|(id, _)| id.clone())
        .collect();
    // Fisher-Yates with the shop's daily RNG.
    for i in (1..ids.len()).rev() {
        let j = ctx.rng.range(0, i as i32 + 1) as usize;
        ids.swap(i, j);
    }
    (ids, random_sale)
}

/// Random-sale price (approx.): several times the base price, at least a
/// random multiple of 100g.
pub fn random_sale_price(db: &ItemDb, item: &Item, ctx: &mut QueryCtx) -> i32 {
    let base = match item {
        Item::Object { id, .. } => db.object(id).map(|o| o.data.price).unwrap_or(0),
        _ => default_buy_price(db, item) / 2,
    };
    let floor = ctx.rng.range(1, 11) * 100;
    floor.max(base * ctx.rng.range(3, 6))
}
