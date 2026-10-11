//! Modal menus layered over the world.

use valley_sim::dialogue::DialoguePage;
use valley_sim::shop::ShopStock;
use valley_sim::world::DayReport;

/// A running arcade game kept in a menu (shared handle so menus stay
/// cloneable).
#[derive(Clone)]
pub struct ArcadeSlot(pub std::rc::Rc<std::cell::RefCell<Box<dyn arcade::ArcadeGame>>>);

impl ArcadeSlot {
    pub fn new(game: Box<dyn arcade::ArcadeGame>) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(game)))
    }
}

impl std::fmt::Debug for ArcadeSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArcadeSlot({})", self.0.borrow().id())
    }
}

impl PartialEq for ArcadeSlot {
    fn eq(&self, o: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &o.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuestionKind {
    Sleep,
    Eat { slot: usize },
    QuitToTitle,
    /// Mine elevator: option i goes to `levels[i]` (last option = cancel).
    Elevator { levels: Vec<i32> },
    FestivalMain,
    FestivalLeave,
    /// Answer an NPC's question (option index is the answer).
    DialogueAnswer,
    /// Jump down a deep-cavern shaft.
    Shaft,
    /// File for divorce.
    Divorce,
    /// Pay a fare and travel.
    Travel { location: String, tile: valley_sim::TilePos, fare: i32 },
    /// Minecart destination `ids[i]` (last option = cancel).
    Minecart { network: String, ids: Vec<String> },
    /// TV channel `channels[i]` (last option = turn off).
    Tv { channels: Vec<valley_sim::minecarts::Channel> },
    /// Claim the mastery perk of `skills[i]` (last option = cancel).
    Mastery { skills: Vec<valley_sim::farmer::Skill> },
    /// A sim choice: option i answers `ids[i]` (last option = cancel).
    Choice { ids: Vec<String> },
    /// A question inside a cutscene (no cancel).
    EventAnswer,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InvTab {
    Items,
    Crafting,
    Skills,
    Social,
    Collections,
    Options,
}

pub const TABS: [InvTab; 6] = [InvTab::Items, InvTab::Crafting, InvTab::Skills, InvTab::Social, InvTab::Collections, InvTab::Options];
pub const TAB_LABELS: [&str; 6] = ["Items", "Crafting", "Skills", "Social", "Collect", "Options"];

#[derive(Clone, Debug, PartialEq)]
pub enum Menu {
    Dialogue {
        npc: Option<String>,
        pages: Vec<DialoguePage>,
        page: usize,
        /// Characters revealed so far (typewriter effect).
        shown: f32,
    },
    Message {
        text: String,
        shown: f32,
    },
    Question {
        text: String,
        options: Vec<String>,
        selected: usize,
        kind: QuestionKind,
    },
    Shop {
        stock: ShopStock,
        scroll: usize,
        hovered: Option<usize>,
    },
    Inventory {
        tab: InvTab,
        /// Slot whose item is "held" on the cursor for swapping.
        held: Option<usize>,
        hovered: Option<usize>,
    },
    DayEnd {
        report: DayReport,
        ms: u32,
    },
    /// The world map of a region.
    WorldMap {
        region: String,
    },
    /// An arcade cabinet.
    Arcade {
        game: ArcadeSlot,
    },
    /// What happened on the farm overnight (shown before the day report).
    NightEvent {
        kind: valley_sim::farm_events::NightEvent,
        ms: u32,
    },
    Chest {
        tile: valley_sim::TilePos,
        hovered: Option<(bool, usize)>,
    },
    Cooking {
        hovered: Option<usize>,
    },
    Letter {
        letter: valley_sim::mail::Letter,
        shown: f32,
    },
    Billboard,
    /// Special orders board.
    OrderBoard,
    /// Sewing machine: inventory slots chosen as fabric (left) and the
    /// second item (right).
    Tailoring { left: Option<usize>, right: Option<usize> },
    /// The forge: item, ingredient (inventory slots).
    Forge { left: Option<usize>, right: Option<usize> },
    /// Casino reels: current bet and the last spin.
    Reels { bet: i32, last: Option<valley_sim::casino::Spin> },
    /// High-Low: bet, the round in play and the last revealed card (card, won).
    HighLow { bet: i32, game: Option<valley_sim::casino::HighLow>, last: Option<(u8, bool)> },
    Journal {
        selected: usize,
    },
    AnimalShop {
        list: Vec<(String, i32)>,
        hovered: Option<usize>,
    },
    Carpenter {
        list: Vec<String>,
        hovered: Option<usize>,
    },
    Blacksmith {
        hovered: Option<usize>,
    },
    LevelUp {
        skill: valley_sim::farmer::Skill,
        level: i32,
        hovered: Option<usize>,
    },
    Bundles {
        area: usize,
        hovered: Option<(usize, usize)>,
    },
    Museum {
        hovered: Option<usize>,
    },
}

/// Characters revealed per tick by the typewriter effect.
pub const TYPE_SPEED: f32 = 1.5;

impl Menu {
    /// Whether the world clock stops while this menu is open (single player).
    pub fn pauses_world(&self) -> bool {
        true
    }
}
