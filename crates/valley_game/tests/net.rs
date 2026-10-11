//! End-to-end co-op over localhost: a client joins, gets a farmhand,
//! receives streamed frames and textures, and its input moves the farmhand.

use engine::{EngineCtx, GameModule, Key, TextureStore};
use valley_content::ContentManager;
use valley_game::net::{InputEvent, NetClient};
use valley_game::ValleyGame;
use valley_sim::world::NewGame;

#[test]
fn client_joins_streams_frames_and_controls_a_farmhand() {
    let saves = std::env::temp_dir().join(format!("valley-net-{}", std::process::id()));
    let mut game = ValleyGame::new(ContentManager::dev_only(), saves);
    game.start_new_game(NewGame { farmer_name: "Host".into(), seed: 5, ..Default::default() });
    let mut ctx = EngineCtx::new(640, 360);
    game.init(&mut ctx);
    let port = game.host("127.0.0.1:0").expect("bind");
    let mut client = NetClient::join(("127.0.0.1", port), "Guest").expect("connect");
    client.push_input(InputEvent::Screen(640, 360));
    let mut store = TextureStore::new();

    let step = |game: &mut ValleyGame, ctx: &mut EngineCtx, client: &mut NetClient, store: &mut TextureStore| {
        game.update(ctx);
        ctx.input.end_tick();
        ctx.tick += 1;
        client.pump(store);
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    for _ in 0..200 {
        step(&mut game, &mut ctx, &mut client, &mut store);
        if client.index.is_some() && client.frame.is_some() {
            break;
        }
    }
    assert_eq!(client.index, Some(0), "welcomed as farmhand 0");
    let w = game.world.as_ref().unwrap();
    assert_eq!(w.farmhands.len(), 1);
    assert_eq!(w.farmhands[0].farmer.name, "Guest");
    let frame = client.frame.clone().expect("a frame arrived");
    assert!(frame.cmds.len() > 20, "the frame has content ({} cmds)", frame.cmds.len());
    assert!(!client.textures.is_empty(), "textures were streamed");

    // Hold "D" (move right) for a while: the farmhand walks, the host doesn't.
    let host_pos = game.world.as_ref().unwrap().farmer.pos;
    let start = game.world.as_ref().unwrap().farmhands[0].farmer.pos;
    client.push_input(InputEvent::KeyDown(Key::D));
    for _ in 0..60 {
        step(&mut game, &mut ctx, &mut client, &mut store);
    }
    client.push_input(InputEvent::KeyUp(Key::D));
    for _ in 0..5 {
        step(&mut game, &mut ctx, &mut client, &mut store);
    }
    let w = game.world.as_ref().unwrap();
    assert!(w.farmhands[0].farmer.pos.x > start.x + 30.0, "farmhand moved right: {:?} -> {:?}", start, w.farmhands[0].farmer.pos);
    assert_eq!(w.farmer.pos, host_pos, "the host stayed put");
    assert!(!w.paused, "co-op never pauses");
}
