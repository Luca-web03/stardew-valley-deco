use valley_content::ContentManager;
use valley_sim::combat::Monster;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

#[test]
fn board_offers_two_orders_and_accepting_one_activates_it() {
    let (_, mut w) = world();
    assert_eq!(w.special_orders.len(), 4);
    w.warp_farmer("Town", TilePos::new(22, 12), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(22, 11))));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenOrderBoard)));
    assert_eq!(w.order_offers.len(), 2);
    let id = w.order_offers[0].clone();
    let (name, text) = w.order_text(&id).unwrap();
    assert!(!name.is_empty() && !text.contains("[LocalizedText"), "tokens expand: {name} / {text}");
    w.accept_order(&id).unwrap();
    assert!(w.farmer.special_order.is_some());
    assert!(!w.order_offers.contains(&id));
    let other = w.order_offers[0].clone();
    assert!(w.accept_order(&other).is_err(), "one order at a time");
}

#[test]
fn delivery_order_completes_with_rewards() {
    let (_, mut w) = world();
    w.accept_order("Dev_TurnipDrive").unwrap();
    let money = w.farmer.money;
    // Deliver in two batches to Tobin.
    let tobin = w.npcs.iter().position(|n| n.name == "Tobin").unwrap();
    let (loc, tile) = (w.npcs[tobin].location.clone(), w.npcs[tobin].tile());
    w.warp_farmer(&loc, TilePos::new(tile.x, tile.y + 1), Some(Dir::Up));
    w.npcs[tobin].set_tile(tile);
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Turnip", 6));
    w.farmer.current_slot = 8;
    assert!(w.do_action(Some(tile)));
    assert_eq!(w.farmer.special_order.as_ref().unwrap().objectives[0].progress, 6);
    assert!(w.farmer.inventory.get(8).is_none());
    w.farmer.inventory.slots[8] = Some(Item::object("Dev_Turnip", 9));
    assert!(w.do_action(Some(tile)));
    assert!(w.farmer.special_order.is_none(), "completed");
    assert_eq!(w.farmer.inventory.get(8).map(|i| i.stack()), Some(5), "only what was needed was taken");
    assert_eq!(w.farmer.money, money + 800);
    assert!(w.farmer.orders_completed.contains(&"Dev_TurnipDrive".to_string()));
    assert!(w.farmer.friendships["Tobin"].points >= 250);
}

#[test]
fn slay_and_mine_floor_objectives_track_progress() {
    let (_, mut w) = world();
    w.accept_order("Dev_CaveCritters").unwrap();
    w.enter_mine_level(1);
    for k in 0..10 {
        w.monsters.clear();
        let info = w.monster_info["Dev_Blob"].clone();
        let here = w.current.clone();
        let mut m = Monster::new(50 + k, info, &here, w.farmer.tile());
        m.health = 1;
        m.info.miss_chance = 0.0;
        m.pos = w.farmer.pos;
        w.monsters.push(m);
        let area = w.farmer.bbox().inflate(64);
        w.hit_area(area, valley_sim::arsenal::Hit { min: 5, max: 5, knockback: 0.0, crit: 0.0, crit_mult: 1.0, ignore_invincible: true });
    }
    assert!(w.farmer.special_order.is_none(), "ten blobs finish the order");
    w.accept_order("Dev_DeepDown").unwrap();
    w.enter_mine_level(4);
    assert_eq!(w.farmer.special_order.as_ref().unwrap().objectives[0].progress, 4);
    w.enter_mine_level(10);
    assert!(w.farmer.special_order.is_none());
}

#[test]
fn orders_expire_and_non_repeatable_ones_do_not_return() {
    let (_, mut w) = world();
    w.accept_order("Dev_FishFry").unwrap();
    for _ in 0..7 {
        assert!(w.farmer.special_order.is_some());
        w.end_day(false);
    }
    assert!(w.farmer.special_order.is_none(), "a week-long order expires");
    // Completing a non-repeatable order removes it from future boards.
    w.farmer.orders_completed.push("Dev_FishFry".into());
    w.farmer.orders_completed.push("Dev_DeepDown".into());
    for _ in 0..3 {
        for _ in 0..7 {
            w.end_day(false);
        }
        assert!(!w.order_offers.iter().any(|o| o == "Dev_FishFry" || o == "Dev_DeepDown"));
    }
}

#[test]
fn active_order_survives_save() {
    let (mut cm, mut w) = world();
    w.accept_order("Dev_TurnipDrive").unwrap();
    w.farmer.special_order.as_mut().unwrap().objectives[0].progress = 3;
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    let o = r.farmer.special_order.as_ref().unwrap();
    assert_eq!(o.id, "Dev_TurnipDrive");
    assert_eq!(o.objectives[0].progress, 3);
}
