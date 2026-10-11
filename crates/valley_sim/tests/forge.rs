use valley_content::ContentManager;
use valley_sim::forge::{monster_class, MonsterClass, GEM_COSTS, RING_COST};
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.farmer.inventory.slots.iter_mut().for_each(|s| *s = None);
    w
}

fn shards(w: &mut World, n: i32) {
    w.farmer.inventory.slots[11] = Some(Item::object("Dev_EmberShard", n));
}

#[test]
fn gems_forge_a_weapon_up_to_three_times() {
    let mut w = world();
    w.farmer.inventory.slots[0] = Some(Item::weapon("Dev_Sword"));
    w.farmer.inventory.slots[1] = Some(Item::object("Dev_Ruby", 4));
    assert!(w.forge(0, 1).is_err(), "no shards");
    shards(&mut w, GEM_COSTS.iter().sum::<i32>() + 50);
    let base = w.weapon_stats(&Item::weapon("Dev_Sword")).unwrap();
    for n in 1..=3 {
        let out = w.forge(0, 1).unwrap();
        assert_eq!(out.weapon_mods().unwrap().gems.len(), n);
    }
    assert!(w.forge(0, 1).is_err(), "three forges at most");
    assert_eq!(w.farmer.inventory.count("Dev_Ruby"), 1);
    assert_eq!(w.farmer.inventory.count("Dev_EmberShard"), 50);
    let forged = w.weapon_stats(w.farmer.inventory.get(0).unwrap()).unwrap();
    assert!(forged.min > base.min && forged.max > base.max);
    let desc = w.items.description(w.farmer.inventory.get(0).unwrap());
    assert!(desc.contains("Forged: Ruby"), "{desc}");

    // Unforging strips the gems and refunds half the shards.
    w.unforge(0).unwrap();
    assert!(w.farmer.inventory.get(0).unwrap().weapon_mods().is_none());
    assert_eq!(w.farmer.inventory.count("Dev_EmberShard"), 50 + GEM_COSTS.iter().sum::<i32>() / 2);
}

#[test]
fn prismatic_shards_enchant_weapons_and_tools() {
    let mut w = world();
    shards(&mut w, 200);
    w.farmer.inventory.slots[0] = Some(Item::weapon("Dev_Sword"));
    w.farmer.inventory.slots[1] = Some(Item::object("Dev_PrismaticShard", 3));
    let first = w.forge(0, 1).unwrap().weapon_mods().unwrap().enchantment.clone().unwrap();
    assert!(valley_sim::forge::WEAPON_ENCHANTS.contains(&first.as_str()));
    let second = w.forge(0, 1).unwrap().weapon_mods().unwrap().enchantment.clone().unwrap();
    assert_ne!(first, second, "rerolls give a different enchantment");

    // A watering can gets a can enchantment; bottomless never runs dry.
    let mut can = Tool::new(ToolKind::WateringCan);
    can.water = 0;
    w.farmer.inventory.slots[2] = Some(Item::Tool(can));
    let out = w.forge(2, 1).unwrap();
    let e = out.tool().unwrap().enchantment.clone().unwrap();
    assert!(["Bottomless", "Efficient"].contains(&e.as_str()), "{e}");
    if let Some(Item::Tool(t)) = w.farmer.inventory.slots[2].as_mut() {
        t.enchantment = Some("Bottomless".into());
    }
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let soil = TilePos::new(20, 21);
    w.location_mut().state.objects.remove(&soil);
    w.location_mut().state.terrain.insert(soil, valley_sim::terrain::TerrainFeature::HoeDirt(Default::default()));
    w.apply_tool(ToolKind::WateringCan, 2, soil);
    assert!(matches!(w.location().terrain(soil), Some(valley_sim::terrain::TerrainFeature::HoeDirt(d)) if d.watered));
    assert_eq!(w.farmer.inventory.get(2).unwrap().tool().unwrap().water, 0);
}

#[test]
fn efficient_tools_cost_no_energy() {
    let mut w = world();
    w.warp_farmer("Farm", TilePos::new(20, 20), Some(Dir::Down));
    let mut axe = Tool::new(ToolKind::Axe);
    axe.enchantment = Some("Efficient".into());
    w.farmer.inventory.slots[0] = Some(Item::Tool(axe));
    w.farmer.current_slot = 0;
    let before = w.farmer.stamina;
    assert!(w.begin_use(None));
    assert_eq!(w.farmer.stamina, before);
}

#[test]
fn rings_combine_and_split() {
    let mut w = world();
    shards(&mut w, RING_COST);
    w.farmer.inventory.slots[0] = Some(Item::object("Dev_GuardRing", 1));
    w.farmer.inventory.slots[1] = Some(Item::object("Dev_MagnetRing", 1));
    let ring = w.forge(0, 1).unwrap();
    assert_eq!(w.items.display_name(&ring), "Combined Ring (Guard Ring + Magnet Ring)");
    assert_eq!(w.farmer.inventory.count("Dev_EmberShard"), 0);
    let slot = w.farmer.inventory.slots.iter().position(|s| s.as_ref() == Some(&ring)).unwrap();
    assert!(w.equip(slot));
    let a = w.equipment_attrs();
    assert_eq!((a.defense, a.magnetic_radius), (2, 2), "both rings work");
    assert!(w.unequip(0));
    let slot = w.farmer.inventory.slots.iter().position(|s| s.as_ref() == Some(&ring)).unwrap();
    w.unforge(slot).unwrap();
    assert_eq!(w.farmer.inventory.count("Dev_GuardRing"), 1);
    assert_eq!(w.farmer.inventory.count("Dev_MagnetRing"), 1);
}

#[test]
fn weapons_take_looks_and_dragon_teeth() {
    let mut w = world();
    shards(&mut w, 100);
    w.farmer.inventory.slots[0] = Some(Item::weapon("Dev_Sword"));
    w.farmer.inventory.slots[1] = Some(Item::weapon("Dev_Club"));
    assert!(w.forge(0, 1).is_err(), "different weapon types");
    w.farmer.inventory.slots[1] = Some(Item::object("Dev_DragonTooth", 1));
    let out = w.forge(0, 1).unwrap();
    assert_eq!(out.weapon_mods().unwrap().innate.len(), 1);
    assert!(w.farmer.inventory.get(1).is_none(), "the tooth is used up");
    // Old saves (weapons without forge data) still load.
    let old: Item = serde_json::from_str(r#"{"Weapon":{"id":"Dev_Sword"}}"#).unwrap();
    assert_eq!(old, Item::weapon("Dev_Sword"));
    let json = serde_json::to_string(&out).unwrap();
    assert_eq!(serde_json::from_str::<Item>(&json).unwrap(), out);
}

#[test]
fn enchantments_know_their_monsters() {
    assert_eq!(monster_class("Dev_Moth"), MonsterClass::Bug);
    assert_eq!(monster_class("Dev_Mummy"), MonsterClass::Undead);
    assert_eq!(monster_class("Dev_Blob"), MonsterClass::Slime);
    assert_eq!(monster_class("Dev_StoneCrab"), MonsterClass::Other);
}
