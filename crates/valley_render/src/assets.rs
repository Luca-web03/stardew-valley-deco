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
