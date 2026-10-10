use valley_content::ContentManager;
use valley_sim::fishing::FishingState;
use valley_sim::input::PlayerInput;
use valley_sim::item::{Item, Tool, ToolKind};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

#[test]
fn cast_hook_play_and_land_a_fish() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    // Stand on the sand above the farm pond (water at x 38..45, y 22..28), facing down.
    w.warp_farmer("Farm", TilePos::new(40, 21), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(40, 21));
    w.farmer.facing = Dir::Down;
    w.farmer.inventory.slots[11] = Some(Item::Tool(Tool::new(ToolKind::FishingRod)));
    w.farmer.current_slot = 11;

    // Press and hold briefly, then release to cast.
    w.update(&PlayerInput { use_tool: true, use_held: true, ..Default::default() });
    for _ in 0..20 {
        w.update(&PlayerInput { use_held: true, ..Default::default() });
    }
    w.update(&PlayerInput::default());
    let mut caught = None;
    for _ in 0..(60 * 60) {
        let input = match &w.fishing {
            FishingState::Nibble { .. } => PlayerInput { use_tool: true, use_held: true, ..Default::default() },
            FishingState::Minigame { game, .. } => {
                let fish = game.fish_pos + 16.0;
                let bar = game.bar_pos + game.bar_height / 2.0;
                let hold = fish < bar - 4.0 || (fish < bar + 8.0 && game.bar_vel > 2.0);
                PlayerInput { use_held: hold, ..Default::default() }
            }
            FishingState::Caught { item, .. } => {
                caught = Some(item.clone());
                PlayerInput::default()
            }
            FishingState::Idle => break,
            _ => PlayerInput::default(),
        };
        w.update(&input);
    }
    let item = caught.expect("caught something");
    let id = item.object_id().unwrap().to_string();
    assert!(id == "Dev_Perch" || id == "Dev_Driftwood", "{id}");
    // The catch lands in the inventory once the popup closes.
    for _ in 0..300 {
        w.update(&PlayerInput::default());
    }
    assert_eq!(w.farmer.inventory.count(&id), 1);
    assert!(w.farmer.stamina < 270.0);
}

#[test]
fn casting_onto_land_fails() {
    let mut cm = ContentManager::dev_only();
    let mut w = World::new(&mut cm, NewGame::default());
    w.warp_farmer("Farm", TilePos::new(20, 12), Some(Dir::Up));
    w.farmer.facing = Dir::Left;
    w.farmer.inventory.slots[11] = Some(Item::Tool(Tool::new(ToolKind::FishingRod)));
    w.farmer.current_slot = 11;
    w.update(&PlayerInput { use_tool: true, use_held: true, ..Default::default() });
    w.update(&PlayerInput::default());
    for _ in 0..60 {
        w.update(&PlayerInput::default());
    }
    assert!(!w.fishing.active());
}
