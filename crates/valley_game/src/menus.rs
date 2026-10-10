//! Modal menus layered over the world.

use valley_sim::dialogue::DialoguePage;
use valley_sim::shop::ShopStock;
use valley_sim::world::DayReport;

#[derive(Clone, Debug, PartialEq)]
pub enum QuestionKind {
    Sleep,
    Eat { slot: usize },
    QuitToTitle,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InvTab {
    Items,
    Skills,
    Social,
    Options,
}

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
}

/// Characters revealed per tick by the typewriter effect.
pub const TYPE_SPEED: f32 = 1.5;

impl Menu {
    /// Whether the world clock stops while this menu is open (single player).
    pub fn pauses_world(&self) -> bool {
        true
    }
}
