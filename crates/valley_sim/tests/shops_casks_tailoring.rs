use valley_content::ContentManager;
use valley_sim::item::{ClothingSlot, Item, Quality};
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::shop::ShopStock;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn open(w: &mut World, id: &str) -> ShopStock {
    w.drain_events();
    w.open_shop(id);
    w.drain_events()
        .into_iter()
        .find_map(|e| match e {
            WorldEvent::OpenShop(s) => Some(s),
            _ => None,
        })
        .unwrap()
}

#[test]
fn trade_items_are_paid_with_items() {
    let mut w = world();
    let stock = open(&mut w, "Dev_DuneTrader");
    let charm = stock.entries.iter().find(|e| e.item.object_id() == Some("Dev_DuneCharm")).unwrap().clone();
    assert_eq!(charm.trade, Some(("Dev_Quartz".to_string(), 3)));
    assert_eq!(charm.price, 0, "trade-only");
    assert!(!w.can_afford(&charm, 1));
    w.farmer.inventory.add(Item::object("Dev_Quartz", 7));
    let money = w.farmer.money;
    assert!(w.buy_from("Dev_DuneTrader", &charm, 2));
    assert_eq!(w.farmer.inventory.count("Dev_Quartz"), 1);
    assert_eq!(w.farmer.inventory.count("Dev_DuneCharm"), 2);
    assert_eq!(w.farmer.money, money, "no gold spent");
    // Money plus items.
    let cloth = stock.entries.iter().find(|e| e.item.object_id() == Some("Dev_Cloth")).unwrap().clone();
    w.farmer.inventory.add(Item::object("Dev_Driftwood", 2));
    assert!(w.buy_from("Dev_DuneTrader", &cloth, 1));
    assert_eq!(w.farmer.money, money - 50);
    assert_eq!(w.farmer.inventory.count("Dev_Driftwood"), 0);
}

#[test]
fn other_currencies_use_their_own_balance() {
    let mut w = world();
    let stock = open(&mut w, "Dev_TokenBooth");
    let hat = stock.entries.iter().find(|e| matches!(e.item, Item::Clothing { slot: ClothingSlot::Hat, .. })).unwrap().clone();
    assert_eq!(hat.currency, 1);
    assert_eq!(valley_sim::shop::currency_name(hat.currency), "tokens");
    w.farmer.money = 100_000;
    assert!(!w.buy(&hat, 1), "gold doesn't count");
    w.farmer.star_tokens = 50;
    assert!(w.buy(&hat, 1));
    assert_eq!(w.farmer.star_tokens, 10);
    assert_eq!(w.farmer.money, 100_000);
}

#[test]
fn casks_age_items_to_higher_quality() {
    let mut w = world();
    let t = TilePos::new(14, 13);
    w.warp_farmer("Farm", TilePos::new(14, 12), Some(Dir::Down));
    w.location_mut().state.objects.remove(&t);
    w.location_mut().state.terrain.remove(&t);
    w.farmer.inventory.slots[4] = Some(Item::big("Dev_Cask", 1));
    w.farmer.current_slot = 4;
    assert!(w.place_current(t));
    assert!(matches!(w.location().object(t).unwrap().kind, ObjKind::Cask { item: None, .. }));
    // Things that don't age are refused; juice ages at 2x.
    w.farmer.inventory.slots[4] = Some(Item::object("Dev_Berry", 1));
    w.drain_events();
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.inventory.count("Dev_Berry"), 1);
    w.farmer.inventory.slots[4] = Some(Item::object("Dev_Juice", 2));
    assert!(w.do_action(Some(t)));
    assert_eq!(w.farmer.inventory.count("Dev_Juice"), 1);
    for _ in 0..7 {
        w.end_day(false);
    }
    w.warp_farmer("Farm", TilePos::new(14, 12), Some(Dir::Down));
    w.farmer.current_slot = 5;
    assert!(w.do_action(Some(t)));
    let silver = w.farmer.inventory.slots.iter().flatten().find(|i| i.object_id() == Some("Dev_Juice") && i.quality() == Quality::Silver);
    assert!(silver.is_some(), "14 base days at 2x = 7 days for silver");
    // Leaving one in long enough reaches iridium and stops there.
    w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_Cask", true, ObjKind::Cask { item: Some(Item::object("Dev_Juice", 1)), aged: 0.0 }));
    for _ in 0..40 {
        w.end_day(false);
    }
    assert!(matches!(w.locations["Farm"].object(t).unwrap().kind, ObjKind::Cask { aged, .. } if aged == valley_sim::casks::AGE_IRIDIUM));
    assert_eq!(valley_sim::casks::aged_quality(56.0), Quality::Iridium);
}

#[test]
fn sewing_turns_cloth_into_clothing() {
    let mut w = world();
    w.farmer.inventory.slots[3] = Some(Item::object("Dev_Cloth", 2));
    w.farmer.inventory.slots[4] = Some(Item::object("Dev_Berry", 1));
    w.farmer.inventory.slots[5] = Some(Item::object("Dev_Turnip", 1));
    assert!(w.tailor(3, 5).is_err(), "no recipe for turnips");
    let shirt = w.tailor(3, 4).unwrap();
    assert_eq!(shirt, Item::Clothing { slot: ClothingSlot::Shirt, id: "Dev_Stripes".into() });
    assert_eq!(w.farmer.inventory.count("Dev_Cloth"), 1);
    assert_eq!(w.farmer.inventory.count("Dev_Berry"), 0, "the berry was used");
    // Recipes that keep the second item.
    w.farmer.inventory.slots[6] = Some(Item::object("Dev_Perch", 1));
    let hat = w.tailor(3, 6).unwrap();
    assert!(matches!(hat, Item::Clothing { slot: ClothingSlot::Hat, .. }));
    assert_eq!(w.farmer.inventory.count("Dev_Perch"), 1);
    assert_eq!(w.farmer.inventory.count("Dev_Cloth"), 0);
}

#[test]
fn sewing_machine_opens_the_tailoring_menu() {
    let mut w = world();
    let t = TilePos::new(14, 13);
    w.warp_farmer("Farm", TilePos::new(14, 12), Some(Dir::Down));
    w.location_mut().state.objects.insert(t, PlacedObject::new("Dev_SewingMachine", true, ObjKind::Placed));
    w.drain_events();
    w.farmer.current_slot = 9;
    assert!(w.do_action(Some(t)));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenTailoring)));
}
