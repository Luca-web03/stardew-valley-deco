use engine::Vec2;

/// World-to-screen transform. `pos` is the world coordinate of the screen's
/// top-left corner.
#[derive(Clone, Copy, Debug, Default)]
pub struct Camera {
    pub pos: Vec2,
    pub screen_w: f32,
    pub screen_h: f32,
}

impl Camera {
    /// Center on `focus`, clamped so the view stays inside the map; maps
    /// smaller than the screen are centered.
    pub fn follow(focus: Vec2, map_w_px: f32, map_h_px: f32, screen_w: f32, screen_h: f32) -> Self {
        let axis = |f: f32, map: f32, screen: f32| -> f32 {
            if map <= screen {
                -(screen - map) / 2.0
            } else {
                (f - screen / 2.0).clamp(0.0, map - screen)
            }
        };
        Self {
            pos: Vec2::new(axis(focus.x, map_w_px, screen_w).round(), axis(focus.y, map_h_px, screen_h).round()),
            screen_w,
            screen_h,
        }
    }

    pub fn to_screen(&self, world: Vec2) -> Vec2 {
        world - self.pos
    }

    pub fn to_world(&self, screen: Vec2) -> Vec2 {
        screen + self.pos
    }

    /// Visible tile range (inclusive) with a margin.
    pub fn tile_range(&self, tile: f32, margin: i32) -> (i32, i32, i32, i32) {
        (
            (self.pos.x / tile).floor() as i32 - margin,
            (self.pos.y / tile).floor() as i32 - margin,
            ((self.pos.x + self.screen_w) / tile).ceil() as i32 + margin,
            ((self.pos.y + self.screen_h) / tile).ceil() as i32 + margin,
        )
    }
}
