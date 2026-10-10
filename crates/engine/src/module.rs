use crate::audio::AudioQueue;
use crate::draw::DrawList;
use crate::input::{InputState, Key};
use crate::texture::TextureStore;

pub const TICKS_PER_SECOND: u32 = 60;
pub const TICK_SECONDS: f64 = 1.0 / TICKS_PER_SECOND as f64;

/// Shared services handed to every module.
pub struct EngineCtx {
    pub textures: TextureStore,
    pub input: InputState,
    pub audio: AudioQueue,
    /// Logical screen size in pixels.
    pub screen_w: u32,
    pub screen_h: u32,
    /// Total fixed ticks elapsed since start.
    pub tick: u64,
    /// Set by a module to ask the host to quit.
    pub quit_requested: bool,
    /// Set by a module to ask the host to switch to another module by id.
    pub switch_to: Option<String>,
}

impl EngineCtx {
    pub fn new(screen_w: u32, screen_h: u32) -> Self {
        Self {
            textures: TextureStore::new(),
            input: InputState::default(),
            audio: AudioQueue::default(),
            screen_w,
            screen_h,
            tick: 0,
            quit_requested: false,
            switch_to: None,
        }
    }
}

/// A game (or game mode) that can be hosted by the engine. Several modules can
/// live in one [`ModuleHost`]; this is how separately ported games are merged
/// into one executable and can hand control to each other.
pub trait GameModule {
    /// Stable identifier, used for switching between modules.
    fn id(&self) -> &str;
    /// Called once before the first update (load textures etc.).
    fn init(&mut self, ctx: &mut EngineCtx);
    /// Advance one fixed tick (1/60 s).
    fn update(&mut self, ctx: &mut EngineCtx);
    /// Emit draw commands for the current state.
    fn draw(&mut self, ctx: &mut EngineCtx, out: &mut DrawList);
    /// Called when the module becomes active again after a switch.
    fn resume(&mut self, _ctx: &mut EngineCtx) {}
    /// Called when another module takes over.
    fn suspend(&mut self, _ctx: &mut EngineCtx) {}
}

/// Owns modules and drives the active one at a fixed tick rate.
pub struct ModuleHost {
    pub ctx: EngineCtx,
    modules: Vec<Box<dyn GameModule>>,
    active: usize,
    initialized: Vec<bool>,
    accumulator: f64,
}

impl ModuleHost {
    pub fn new(screen_w: u32, screen_h: u32) -> Self {
        Self {
            ctx: EngineCtx::new(screen_w, screen_h),
            modules: Vec::new(),
            active: 0,
            initialized: Vec::new(),
            accumulator: 0.0,
        }
    }

    pub fn add(&mut self, m: Box<dyn GameModule>) {
        self.modules.push(m);
        self.initialized.push(false);
    }

    pub fn module_ids(&self) -> Vec<String> {
        self.modules.iter().map(|m| m.id().to_string()).collect()
    }

    pub fn active_id(&self) -> Option<&str> {
        self.modules.get(self.active).map(|m| m.id())
    }

    fn ensure_init(&mut self) {
        if let Some(false) = self.initialized.get(self.active).copied() {
            self.modules[self.active].init(&mut self.ctx);
            self.initialized[self.active] = true;
        }
    }

    pub fn switch_to(&mut self, id: &str) -> bool {
        let Some(idx) = self.modules.iter().position(|m| m.id() == id) else {
            return false;
        };
        if idx == self.active {
            return true;
        }
        self.ensure_init();
        self.modules[self.active].suspend(&mut self.ctx);
        self.active = idx;
        self.ctx.input.reset();
        let was_init = self.initialized[idx];
        self.ensure_init();
        if was_init {
            self.modules[idx].resume(&mut self.ctx);
        }
        true
    }

    /// Run exactly one fixed tick.
    pub fn tick(&mut self) {
        if self.modules.is_empty() {
            return;
        }
        self.ensure_init();
        // Global module switcher: Ctrl+Tab cycles hosted games.
        if self.modules.len() > 1 && self.ctx.input.held(Key::LeftCtrl) && self.ctx.input.pressed(Key::Tab) {
            let next = (self.active + 1) % self.modules.len();
            let id = self.modules[next].id().to_string();
            self.switch_to(&id);
        }
        self.modules[self.active].update(&mut self.ctx);
        self.ctx.tick += 1;
        self.ctx.input.end_tick();
        if let Some(id) = self.ctx.switch_to.take() {
            self.switch_to(&id);
        }
    }

    /// Feed real elapsed time; runs as many fixed ticks as needed (capped).
    pub fn advance(&mut self, dt: f64) -> u32 {
        self.accumulator += dt.min(0.25);
        let mut n = 0;
        while self.accumulator >= TICK_SECONDS {
            self.accumulator -= TICK_SECONDS;
            self.tick();
            n += 1;
        }
        n
    }

    pub fn draw(&mut self, out: &mut DrawList) {
        if self.modules.is_empty() {
            return;
        }
        self.ensure_init();
        out.width = self.ctx.screen_w;
        out.height = self.ctx.screen_h;
        self.modules[self.active].draw(&mut self.ctx, out);
    }
}
