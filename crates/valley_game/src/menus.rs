//! Modal menus layered over the world.

use valley_sim::dialogue::DialoguePage;
use valley_sim::shop::ShopStock;
use valley_sim::world::DayReport;

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
