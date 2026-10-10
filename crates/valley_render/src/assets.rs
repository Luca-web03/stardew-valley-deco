use std::collections::HashMap;

use engine::{Color, Image, TexId, TextureStore};
use valley_content::ContentManager;

/// Lazily loads content textures into the engine texture store.
#[derive(Default)]
pub struct Assets {
    cache: HashMap<String, TexId>,
    missing: Option<TexId>,
    shadow: Option<TexId>,
    white: Option<TexId>,
    ladder: Option<TexId>,
    layouts: Option<crate::layouts::Layouts>,
}

impl Assets {
    pub fn new() -> Self {
        Self::default()
    }

    fn missing_tex(&mut self, store: &mut TextureStore) -> TexId {
        *self.missing.get_or_insert_with(|| {
            let mut img = Image::new(16, 16);
            for y in 0..16 {
                for x in 0..16 {
                    let c = if (x / 4 + y / 4) % 2 == 0 { Color::rgb(255, 0, 255) } else { Color::BLACK };
                    img.set(x, y, c);
                }
            }
            store.insert("__missing", img)
        })
    }

    /// Soft elliptical shadow (16x6).
    pub fn shadow(&mut self, store: &mut TextureStore) -> TexId {
        *self.shadow.get_or_insert_with(|| {
            let mut img = Image::new(16, 6);
            for y in 0..6 {
                for x in 0..16 {
                    let dx = (x as f32 + 0.5 - 8.0) / 8.0;
                    let dy = (y as f32 + 0.5 - 3.0) / 3.0;
                    if dx * dx + dy * dy <= 1.0 {
                        img.set(x, y, Color::rgba(0, 0, 0, 90));
                    }
                }
            }
            store.insert("__shadow", img)
        })
    }

    /// Sprite-sheet layouts (loaded once from the content folder).
    pub fn layouts(&mut self, cm: &mut ContentManager) -> crate::layouts::Layouts {
        self.layouts.get_or_insert_with(|| crate::layouts::Layouts::load(cm)).clone()
    }

    /// Engine-owned ladder sprites (16x16: down ladder at x 0, up ladder at x 16).
    pub fn ladder(&mut self, store: &mut TextureStore) -> TexId {
        *self.ladder.get_or_insert_with(|| {
            let mut img = Image::new(32, 16);
            let hole = Color::rgb(20, 14, 12);
            let wood = Color::rgb(150, 100, 56);
            for y in 2..15 {
                for x in 2..14 {
                    img.set(x, y, hole);
                }
            }
            for ox in [0, 16] {
                for y in 1..16 {
                    img.set(ox + 4, y, wood);
                    img.set(ox + 11, y, wood);
                }
                for y in (3..16).step_by(4) {
                    for x in 4..12 {
                        img.set(ox + x, y, wood);
                    }
                }
            }
            for y in 0..16 {
                for x in 16..32 {
                    if img.get(x, y) == hole {
                        img.set(x as i32, y as i32, Color::TRANSPARENT);
                    }
                }
            }
            store.insert("__ladder", img)
        })
    }

    /// 1x1 white texture for tinted quads.
    pub fn white(&mut self, store: &mut TextureStore) -> TexId {
        *self.white.get_or_insert_with(|| store.insert("__white", Image::filled(1, 1, Color::WHITE)))
    }

    /// Texture for a game asset name (e.g. "Maps/springobjects").
    pub fn tex(&mut self, cm: &mut ContentManager, store: &mut TextureStore, asset: &str) -> TexId {
        let key = asset.replace('\\', "/");
        if let Some(&t) = self.cache.get(&key) {
            return t;
        }
        let id = match cm.load_image(&key) {
            Some(img) => store.insert(key.clone(), img),
            None => self.missing_tex(store),
        };
        self.cache.insert(key, id);
        id
    }

    /// A texture generated in code, cached by name.
    pub fn generated(&mut self, store: &mut TextureStore, name: &str, make: impl FnOnce() -> Image) -> TexId {
        if let Some(&t) = self.cache.get(name) {
            return t;
        }
        let id = store.insert(name.to_string(), make());
        self.cache.insert(name.to_string(), id);
        id
    }

    /// Like [`Assets::tex`] but returns None if the asset doesn't exist.
    pub fn try_tex(&mut self, cm: &mut ContentManager, store: &mut TextureStore, asset: &str) -> Option<TexId> {
        let t = self.tex(cm, store, asset);
        (Some(t) != self.missing).then_some(t)
    }

    /// Tilesheet image referenced from a map (relative to Maps/).
    pub fn map_sheet(&mut self, cm: &mut ContentManager, store: &mut TextureStore, image: &str) -> TexId {
        let maps = format!("Maps/{image}");
        if let Some(t) = self.try_tex(cm, store, &maps) {
            return t;
        }
        self.tex(cm, store, image)
    }
}
