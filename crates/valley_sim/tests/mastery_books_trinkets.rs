use valley_content::ContentManager;
use valley_sim::farmer::{Skill, XP_LEVELS};
use valley_sim::item::Item;
use valley_sim::mastery::SKILLS;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

#[test]
fn mastery_points_come_from_xp_past_level_ten() {
    let (_, mut w) = world();
    assert_eq!(w.mastery_level(), 0);
    assert!(w.claim_mastery(Skill::Farming).is_err());
    for s in SKILLS {
        w.farmer.skills.xp.insert(s, XP_LEVELS[9]);
    }
    assert_eq!(w.mastery_xp(), 0);
    w.farmer.skills.gain(Skill::Fishing, 26_000);
    assert_eq!(w.mastery_level(), 2);
    assert_eq!(w.mastery_points(), 2);
    let stamina = w.farmer.max_stamina;
    assert!(w.claim_mastery(Skill::Foraging).is_ok());
    assert_eq!(w.farmer.max_stamina, stamina + 20);
    assert!(w.claim_mastery(Skill::Foraging).is_err(), "once per skill");
    assert!(w.claim_mastery(Skill::Combat).is_ok());
    assert!(w.claim_mastery(Skill::Mining).is_err(), "out of points");
    // The pedestal offers the rest.
    w.warp_farmer("Town", TilePos::new(4, 12), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(4, 11))));
    let skills = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::MasteryMenu { skills, .. } => Some(skills),
        _ => None,
    });
    assert_eq!(skills.unwrap().len(), 3);
}

#[test]
fn books_teach_skills_and_powers() {
    let (mut cm, mut w) = world();
    w.farmer.inventory.slots[7] = Some(Item::object("Dev_BookGarden", 1));
    w.farmer.current_slot = 7;
    let xp = w.farmer.skills.xp(Skill::Farming);
    assert!(w.do_action(None));
    assert_eq!(w.farmer.skills.xp(Skill::Farming), xp + 250);
    assert!(w.farmer.inventory.get(7).is_none(), "read and put away");
    w.farmer.inventory.slots[7] = Some(Item::object("Dev_BookDunes", 2));
    assert!(w.do_action(None));
    assert_eq!(w.farmer.powers, vec!["Dev_DuneWalker"]);
    w.drain_events();
    assert!(w.do_action(None));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("already know"))));
    assert_eq!(w.farmer.powers.len(), 1);
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.farmer.powers, w.farmer.powers);
}

#[test]
fn trinkets_equip_and_add_stats() {
    let (_, mut w) = world();
    let charm = w.items.create("(TR)Dev_PebbleCharm", 1).unwrap();
    assert_eq!(charm.qualified_id(), "(TR)Dev_PebbleCharm");
    assert_eq!(w.items.display_name(&charm), "Pebble Charm");
    let defense = w.equipment_attrs().defense;
    w.farmer.inventory.slots[9] = Some(charm);
    assert!(w.equip(9));
    assert_eq!(w.equipment_attrs().defense, defense + 2);
    w.farmer.inventory.slots[9] = Some(Item::Trinket { id: "Dev_SwiftFeather".into() });
    assert!(w.equip(9), "swaps the charm back out");
    assert_eq!(w.farmer.inventory.get(9), Some(&Item::Trinket { id: "Dev_PebbleCharm".into() }));
    let a = w.equipment_attrs();
    assert_eq!((a.speed, a.attack, a.defense), (1, 1, defense));
    assert!(w.unequip(6));
    assert!(w.farmer.trinket.is_none());
}
