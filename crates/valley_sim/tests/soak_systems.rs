//! A second long-running smoke test covering the newer systems together:
//! remixed bundles, fences and paths, bushes, bee houses, sprinkler
//! attachments, automation, forged gear, decorating, swimming, trigger
//! actions — over two years with save/reload every season.

use valley_content::ContentManager;
use valley_sim::input::PlayerInput;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::objects::{ObjKind, PlacedObject, SprinklerShape};
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::TilePos;

#[test]
fn two_busy_years_with_everything_running() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame { remixed_bundles: true, seed: 99, ..Default::default() });
    w.farmer.money = 500_000;
    w.farmer.inventory.add(Item::object("388", 999));
    w.farmer.inventory.add(Item::object("390", 999));
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for y in 15..30 {
            for x in 5..40 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
        farm.state.clumps.clear();
        // Fences, a gate, a path, a bee house, sprinklers with attachments.
        for x in 20..30 {
            farm.state.objects.insert(TilePos::new(x, 28), PlacedObject::new("Dev_WoodFence", false, ObjKind::Fence { health: 28, open: false }));
        }
        farm.state.objects.insert(TilePos::new(25, 28), PlacedObject::new("Dev_Gate", false, ObjKind::Fence { health: 1, open: false }));
        for x in 20..30 {
            farm.state.terrain.insert(TilePos::new(x, 26), valley_sim::terrain::TerrainFeature::Flooring("Dev_StonePath".into()));
        }
        farm.state.objects.insert(TilePos::new(32, 20), PlacedObject::new("Dev_BeeHouse", true, ObjKind::Machine { output: None, minutes_left: 0, ready: false }));
        farm.state.objects.insert(
            TilePos::new(15, 22),
            PlacedObject::new("Dev_Sprinkler", false, ObjKind::Sprinkler { radius: SprinklerShape::Cross, attachment: Some("Dev_Enricher".into()), fertilizer: Some(Item::object("Dev_Fertilizer", 50)) }),
        );
        for (dx, dy) in [(0, 1), (1, 0), (0, -1), (-1, 0)] {
            let p = TilePos::new(15 + dx, 22 + dy);
            let mut dirt = valley_sim::terrain::HoeDirt::new();
            dirt.crop = Some(valley_sim::crop::Crop::new("Dev_TurnipSeeds", &mut engine::Rng::new((dx + 7) as u64)));
            farm.state.terrain.insert(p, valley_sim::terrain::TerrainFeature::HoeDirt(dirt));
        }
        let mut tea = valley_sim::bushes::Bush::new(valley_sim::bushes::TEA);
        tea.planted = 0;
        farm.state.terrain.insert(TilePos::new(10, 18), valley_sim::terrain::TerrainFeature::Bush(tea));
    }
    w.place_building("Coop", TilePos::new(6, 16)).unwrap();
    w.end_day(false);
    w.buy_animal("Dev_Hen", None).unwrap();
    let coop = w.buildings[0].interior.clone().unwrap();
    w.locations.get_mut(&coop).unwrap().state.objects.insert(TilePos::new(2, 5), PlacedObject::new("Dev_AutoGrabber", true, ObjKind::Chest(valley_sim::inventory::Inventory::new(36))));
    w.locations.get_mut(&coop).unwrap().state.objects.insert(TilePos::new(3, 5), PlacedObject::new("Dev_AutoPetter", true, ObjKind::Placed));
    // A forged, enchanted sword in hand and an enchanted can.
    let mut sword = Item::weapon("Dev_Sword");
    if let Item::Weapon { mods, .. } = &mut sword {
        *mods = Some(Box::new(valley_sim::item::WeaponMods { gems: vec!["Ruby".into()], enchantment: Some("Vampiric".into()), ..Default::default() }));
    }
    w.farmer.inventory.slots[10] = Some(sword);
    let mut can = Tool::new(ToolKind::WateringCan);
    can.enchantment = Some("Bottomless".into());
    w.farmer.inventory.slots[2] = Some(Item::Tool(can));
    // Paint the farmhouse.
    w.locations.get_mut("FarmHouse").unwrap().state.paint.flooring.insert("1 3".into(), "2".into());

    for day in 0..224 {
        for t in 0..90u32 {
            let input = PlayerInput { move_x: [1, 0, -1, 0][(t as usize / 20) % 4], move_y: [0, 1, 0, -1][(t as usize / 20) % 4], use_tool: t % 30 == 0, action: t % 45 == 0, ..Default::default() };
            w.farmer.current_slot = [10, 2, 0][(day as usize) % 3];
            w.update(&input);
        }
        if day % 17 == 0 {
            w.warp_farmer("Dev_Spa", TilePos::new(3, 3), None);
            w.update(&PlayerInput::default());
            w.warp_farmer("Farm", TilePos::new(20, 20), None);
        }
        w.drain_events();
        w.end_day(false);
        w.drain_events();
        assert!(w.farmer.health >= 1 && w.farmer.money >= 0, "day {day}");
        if day % 28 == 27 {
            let json = SaveData::capture(&w).to_json();
            let restored = SaveData::from_json(&json).expect("save parses").restore(&mut cm);
            assert_eq!(restored.farmer.bundle_data, w.farmer.bundle_data);
            assert_eq!(restored.bundles.len(), w.bundles.len());
            assert_eq!(restored.locations["FarmHouse"].state.paint, w.locations["FarmHouse"].state.paint);
            assert_eq!(restored.farmer.inventory.slots[10], w.farmer.inventory.slots[10]);
            w = restored;
        }
    }
    // Fences fell over with time; the gate stands.
    let farm = &w.locations["Farm"];
    assert!(matches!(farm.object(TilePos::new(20, 28)).map(|o| &o.kind), Some(ObjKind::Fence { health: 0, .. })));
    assert!(matches!(farm.object(TilePos::new(25, 28)).map(|o| &o.kind), Some(ObjKind::Fence { health: 1, .. })));
    // The auto-grabber has been busy.
    let ObjKind::Chest(inv) = &w.locations[&coop].object(TilePos::new(2, 5)).unwrap().kind else { panic!() };
    assert!(inv.slots.iter().flatten().count() > 0, "eggs gathered");
}
