use valley_content::data::{QuantityModifier, ShopData, ShopItemData};
use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World, WorldEvent};

fn open(w: &mut World, id: &str) -> valley_sim::shop::ShopStock {
    w.drain_events();
    w.open_shop(id);
    w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::OpenShop(s) => Some(s),
        _ => None,
    }).unwrap()
}

#[test]
fn shop_modifiers_conditions_actions_and_recipes() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    let item = |id: &str, price: i32| ShopItemData { id: id.into(), item_id: format!("(O){id}"), price, ..Default::default() };
    let mut shop = ShopData::default();
    let mut turnip = item("Dev_Turnip", 100);
    turnip.price_modifiers = Some(vec![QuantityModifier { modification: "Multiply".into(), amount: 2.0, ..Default::default() }]);
    turnip.actions_on_purchase = Some(vec!["AddMail Current Dev_Bought received".into()]);
    let mut bread = item("Dev_Bread", 60);
    bread.ignore_shop_price_modifiers = true;
    let mut salad = item("Dev_Salad", 500);
    salad.is_recipe = true;
    let mut seeds = ShopItemData { id: "all".into(), item_id: "ALL_ITEMS (O)".into(), price: 1, per_item_condition: Some("ITEM_CATEGORY Target -74".into()), ..Default::default() };
    seeds.avoid_repeat = true;
    shop.items = vec![turnip, bread, salad, seeds];
    shop.price_modifiers = Some(vec![QuantityModifier { modification: "Subtract".into(), amount: 10.0, ..Default::default() }]);
    w.shops.insert("Test".into(), shop);
    let stock = open(&mut w, "Test");
    let price = |id: &str| stock.entries.iter().find(|e| e.item.object_id() == Some(id)).map(|e| e.price);
    assert_eq!(price("Dev_Turnip"), Some(190), "item x2 then shop -10");
    assert_eq!(price("Dev_Bread"), Some(60), "ignores the shop modifier");
    assert!(stock.entries.iter().filter(|e| e.price == 0 || e.price == 1).all(|e| e.item.object_id().and_then(|i| w.items.object(i)).is_some_and(|o| o.data.category == -74)), "per-item condition: seeds only");
    w.farmer.money = 10_000;
    let turnip = stock.entries.iter().find(|e| e.item.object_id() == Some("Dev_Turnip")).unwrap().clone();
    assert!(w.buy(&turnip, 1));
    assert!(w.farmer.mail_received.contains(&"Dev_Bought".to_string()));
    let salad = stock.entries.iter().find(|e| e.item.object_id() == Some("Dev_Salad")).unwrap().clone();
    w.farmer.cooking_recipes.remove("Garden Salad");
    assert!(w.buy(&salad, 1));
    assert!(w.farmer.cooking_recipes.contains_key("Garden Salad"));
    assert_eq!(w.farmer.inventory.count("Dev_Salad"), 0, "recipes aren't items");
    // Selling: the general store buys produce, not ore.
    assert!(w.shop_buys("SeedShop", &Item::object("Dev_Turnip", 1)));
    assert!(!w.shop_buys("SeedShop", &Item::object("Dev_CopperOre", 1)));
    assert!(!w.shop_buys("Furniture", &Item::object("Dev_Turnip", 1)));
}
