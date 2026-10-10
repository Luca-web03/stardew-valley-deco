use valley_content::ContentManager;
use valley_sim::item::{ClothingSlot, Item};
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

#[test]
fn clothing_loads_from_data_with_qualified_ids() {
    let (_, w) = world();
    let hat = w.items.create("(H)Dev_StrawHat", 1).unwrap();
    assert_eq!(hat, Item::Clothing { slot: ClothingSlot::Hat, id: "Dev_StrawHat".into() });
    assert_eq!(hat.qualified_id(), "(H)Dev_StrawHat");
    assert_eq!(w.items.display_name(&hat), "Straw Hat");
    let shirt = w.items.create("(S)Dev_Stripes", 5).unwrap();
    assert_eq!(shirt.stack(), 1, "clothing doesn't stack");
    assert_eq!(w.items.clothing(ClothingSlot::Shirt, "Dev_Stripes").unwrap().color, Some([70, 120, 190]));
    assert!(w.items.hats["Dev_Beanie"].hides_hair);
    assert!(w.items.context_tags(&shirt).iter().any(|t| t == "category_shirt"));
    assert!(w.items.create("(P)Nope", 1).is_none());
}

#[test]
fn wearing_clothes_changes_the_look_and_swaps_back() {
    let (_, mut w) = world();
    let base = w.worn_appearance();
    w.farmer.inventory.slots[10] = Some(Item::Clothing { slot: ClothingSlot::Shirt, id: "Dev_Stripes".into() });
    assert!(w.equip(10));
    assert!(w.farmer.inventory.get(10).is_none());
    assert_eq!(w.worn_appearance().shirt, [70, 120, 190]);
    assert_eq!(w.worn_appearance().pants, base.pants);
    // Equipping another shirt swaps the first back into the slot.
    w.farmer.inventory.slots[10] = Some(Item::Clothing { slot: ClothingSlot::Shirt, id: "Dev_Tee".into() });
    assert!(w.equip(10));
    assert_eq!(w.farmer.inventory.get(10), Some(&Item::Clothing { slot: ClothingSlot::Shirt, id: "Dev_Stripes".into() }));
    // Hats and pants go to their own slots.
    w.farmer.inventory.slots[11] = Some(Item::Clothing { slot: ClothingSlot::Hat, id: "Dev_Cap".into() });
    w.farmer.inventory.slots[12] = Some(Item::Clothing { slot: ClothingSlot::Pants, id: "Dev_WorkPants".into() });
    assert!(w.equip(11) && w.equip(12));
    assert!(w.farmer.hat.is_some() && w.farmer.pants_item.is_some());
    assert_eq!(w.worn_appearance().pants, [90, 80, 70]);
    // Unequip the hat (slot index 3) back into the inventory.
    assert!(w.unequip(3));
    assert!(w.farmer.hat.is_none());
    let hats = w.farmer.inventory.slots.iter().flatten().filter(|i| matches!(i, Item::Clothing { slot: ClothingSlot::Hat, .. })).count();
    assert_eq!(hats, 1);
}

#[test]
fn tailor_sells_seasonal_clothing_and_outfits_save() {
    let (mut cm, mut w) = world();
    w.drain_events();
    w.open_shop("Dev_Tailor");
    let stock = w
        .drain_events()
        .into_iter()
        .find_map(|e| match e {
            valley_sim::world::WorldEvent::OpenShop(s) => Some(s),
            _ => None,
        })
        .unwrap();
    let ids: Vec<String> = stock.entries.iter().map(|e| e.item.qualified_id()).collect();
    assert!(ids.contains(&"(H)Dev_StrawHat".to_string()));
    assert!(!ids.contains(&"(H)Dev_Beanie".to_string()), "beanies are for fall and winter");
    let tee = stock.entries.iter().find(|e| e.item.qualified_id() == "(S)Dev_Tee").unwrap();
    assert_eq!(tee.price, 100, "default price from the data");
    w.farmer.hat = Some(Item::Clothing { slot: ClothingSlot::Hat, id: "Dev_StrawHat".into() });
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert_eq!(r.farmer.hat, w.farmer.hat);
}
