use engine::Rng;
use valley_content::ContentManager;
use valley_sim::fishing::{Minigame, TRACK};
use valley_sim::fishing_gear::{Gear, TackleEffect, TACKLE_USES};
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::world::{NewGame, World};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn rod(level: i32) -> Item {
    let mut t = Tool::new(ToolKind::FishingRod);
    t.level = level;
    Item::Tool(t)
}

#[test]
fn bait_and_tackle_attach_to_upgraded_rods() {
    let mut w = world();
    w.farmer.inventory.slots[2] = Some(rod(0));
    w.farmer.inventory.slots[3] = Some(Item::object("Dev_Bait", 10));
    assert!(!w.attach_to_rod(3, 2), "a basic rod has no bait slot");
    w.farmer.inventory.slots[2] = Some(rod(1));
    assert!(w.attach_to_rod(3, 2));
    assert!(w.farmer.inventory.get(3).is_none());
    w.farmer.inventory.slots[4] = Some(Item::object("Dev_WhirlLure", 2));
    assert!(!w.attach_to_rod(4, 2), "tackle needs a better rod");
    w.farmer.inventory.slots[2] = Some(match w.farmer.inventory.get(2).cloned() {
        Some(Item::Tool(mut t)) => {
            t.level = 2;
            Item::Tool(t)
        }
        _ => unreachable!(),
    });
    assert!(w.attach_to_rod(4, 2));
    assert_eq!(w.farmer.inventory.count("Dev_WhirlLure"), 1, "one piece of tackle goes on");
    w.farmer.current_slot = 2;
    assert_eq!(w.current_gear(), Gear { bait: true, tackle: Some(TackleEffect::BiteSpeed) });
    // Hooking a fish uses a bait and a tackle use.
    w.consume_gear_pub();
    let Some(Item::Tool(t)) = w.farmer.inventory.get(2).cloned() else { panic!() };
    assert_eq!(t.attachments[0].as_ref().unwrap().stack(), 9);
    assert_eq!(t.tackle_uses, TACKLE_USES - 1);
    // Non-gear doesn't attach (and the UI swaps instead).
    w.farmer.inventory.slots[5] = Some(Item::object("Dev_Turnip", 1));
    assert!(!w.attach_to_rod(5, 2));
}

#[test]
fn tackle_effects_change_the_minigame() {
    let w = world();
    let fish = w.fish_db().values().find(|f| f.difficulty > 0).unwrap().clone();
    let plain = Minigame::new(fish.clone(), 0);
    let cork = Minigame::with_gear(fish.clone(), 0, Gear { bait: false, tackle: Some(TackleEffect::BarSize) }, None);
    assert!(cork.bar_height > plain.bar_height);
    // Treasure surfaces and can be grabbed by keeping it in the bar.
    let mut g = Minigame::with_gear(fish, 0, Gear { bait: false, tackle: Some(TackleEffect::Treasure) }, Some(0));
    let mut rng = Rng::new(5);
    for _ in 0..2000 {
        if g.treasure_caught {
            break;
        }
        let target = g.treasure.map(|(p, _)| p).unwrap_or(TRACK / 2.0);
        let center = g.bar_pos + g.bar_height / 2.0;
        let r = g.step(center > target + 12.0, &mut rng, 16);
        if r.is_some() {
            g.progress = 0.5;
        }
    }
    assert!(g.treasure_caught, "treasure grabbed");
}
