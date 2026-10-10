use std::collections::HashMap;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::map::Map;
use crate::source::{normalize, ContentSource, DirSource, LayeredSource};
use crate::xnb::{self, XnbAsset};

/// Loads and caches assets by game asset name (e.g. `Maps/Farm`,
/// `Data/Objects`, `TileSheets/crops`).
pub struct ContentManager {
    source: Box<dyn ContentSource>,
    json_cache: HashMap<String, Arc<serde_json::Value>>,
    string_cache: HashMap<String, HashMap<String, String>>,
    pub warnings: Vec<String>,
    /// Language suffix for localized assets, e.g. "de-DE". Empty = default.
    pub locale: String,
    /// True if any user-provided (real game) content directory is mounted.
    pub has_game_content: bool,
}

impl ContentManager {
    pub fn new(source: Box<dyn ContentSource>) -> Self {
        Self {
            source,
            json_cache: HashMap::new(),
            string_cache: HashMap::new(),
            warnings: Vec::new(),
            locale: String::new(),
            has_game_content: false,
        }
    }

    /// User content directory (if any) layered over the built-in placeholder pack.
    pub fn with_game_dir(dir: Option<&std::path::Path>) -> Self {
        let mut layered = LayeredSource::default();
        let mut has = false;
        if let Some(d) = dir {
            layered.push(Box::new(DirSource::new(d)));
            has = true;
        }
        layered.push(Box::new(crate::dev::build()));
        let mut cm = Self::new(Box::new(layered));
        cm.has_game_content = has;
        cm
    }

    /// Placeholder pack only.
    pub fn dev_only() -> Self {
        Self::with_game_dir(None)
    }

    pub fn describe(&self) -> String {
        self.source.describe()
    }

    fn read_with_ext(&self, asset: &str, exts: &[&str]) -> Option<(String, Vec<u8>)> {
        let base = normalize(asset);
        for ext in exts {
            if !self.locale.is_empty() {
                let p = format!("{base}.{}.{ext}", self.locale);
                if let Some(b) = self.source.read(&p) {
                    return Some((ext.to_string(), b));
                }
            }
            let p = format!("{base}.{ext}");
            if let Some(b) = self.source.read(&p) {
                return Some((ext.to_string(), b));
            }
        }
        None
    }

    pub fn exists(&self, asset: &str) -> bool {
        self.read_with_ext(asset, &["png", "json", "tmx", "tbin", "xnb"]).is_some()
    }

    pub fn load_image(&mut self, asset: &str) -> Option<engine::Image> {
        let (ext, bytes) = self.read_with_ext(asset, &["png", "xnb"])?;
        let r = match ext.as_str() {
            "png" => engine::Image::decode_png(&bytes),
            _ => match xnb::read(&bytes) {
                Ok(XnbAsset::Texture(img)) => Ok(img),
                Ok(_) => Err("not a texture".into()),
                Err(e) => Err(e),
            },
        };
        match r {
            Ok(i) => Some(i),
            Err(e) => {
                self.warnings.push(format!("{asset}: {e}"));
                None
            }
        }
    }

    pub fn load_map(&mut self, asset: &str) -> Option<Map> {
        let (ext, bytes) = self.read_with_ext(asset, &["tmx", "tbin", "xnb"])?;
        let id = normalize(asset).rsplit('/').next().unwrap_or(asset).to_string();
        let r = match ext.as_str() {
            "tmx" => crate::tmx::parse(&String::from_utf8_lossy(&bytes), &id),
            "tbin" => crate::tbin::parse(&bytes, &id),
            _ => match xnb::read(&bytes) {
                Ok(XnbAsset::Tbin(t)) => crate::tbin::parse(&t, &id),
                Ok(_) => Err("not a map".into()),
                Err(e) => Err(e),
            },
        };
        match r {
            Ok(m) => Some(m),
            Err(e) => {
                self.warnings.push(format!("{asset}: {e}"));
                None
            }
        }
    }

    pub fn load_json_value(&mut self, asset: &str) -> Option<Arc<serde_json::Value>> {
        let key = normalize(asset).to_lowercase();
        if let Some(v) = self.json_cache.get(&key) {
            return Some(v.clone());
        }
        let (_, bytes) = self.read_with_ext(asset, &["json"])?;
        // Tolerate a UTF-8 BOM.
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
        match serde_json::from_slice::<serde_json::Value>(bytes) {
            Ok(v) => {
                let v = Arc::new(v);
                self.json_cache.insert(key, v.clone());
                Some(v)
            }
            Err(e) => {
                self.warnings.push(format!("{asset}: {e}"));
                None
            }
        }
    }

    pub fn load_json<T: DeserializeOwned>(&mut self, asset: &str) -> Option<T> {
        let v = self.load_json_value(asset)?;
        match serde_json::from_value::<T>((*v).clone()) {
            Ok(t) => Some(t),
            Err(e) => {
                self.warnings.push(format!("{asset}: {e}"));
                None
            }
        }
    }

    /// Load a `Dictionary<string, T>` asset, skipping entries that fail to parse.
    pub fn load_dict<T: DeserializeOwned>(&mut self, asset: &str) -> std::collections::BTreeMap<String, T> {
        let mut out = std::collections::BTreeMap::new();
        let Some(v) = self.load_json_value(asset) else { return out };
        if let Some(obj) = v.as_object() {
            for (k, val) in obj {
                match serde_json::from_value::<T>(val.clone()) {
                    Ok(t) => {
                        out.insert(k.clone(), t);
                    }
                    Err(e) => self.warnings.push(format!("{asset}[{k}]: {e}")),
                }
            }
        }
        out
    }

    /// `Dictionary<string,string>` assets (string tables, dialogue, schedules...).
    pub fn load_string_dict(&mut self, asset: &str) -> HashMap<String, String> {
        let key = normalize(asset).to_lowercase();
        if let Some(m) = self.string_cache.get(&key) {
            return m.clone();
        }
        let mut m = HashMap::new();
        if let Some(v) = self.load_json_value(asset) {
            if let Some(obj) = v.as_object() {
                for (k, val) in obj {
                    if let Some(s) = val.as_str() {
                        m.insert(k.clone(), s.to_string());
                    }
                }
            }
        }
        self.string_cache.insert(key, m.clone());
        m
    }

    /// Look up `asset:key` in a string table.
    pub fn string(&mut self, asset: &str, key: &str) -> Option<String> {
        let key = key.to_string();
        let asset = normalize(asset);
        if !self.string_cache.contains_key(&asset.to_lowercase()) {
            self.load_string_dict(&asset);
        }
        self.string_cache.get(&asset.to_lowercase()).and_then(|m| m.get(&key).cloned())
    }

    /// Expand tokenized text like `[LocalizedText Strings\Objects:X]`.
    pub fn expand(&mut self, text: &str) -> String {
        if !text.contains('[') {
            return text.to_string();
        }
        let mut look = |asset: &str, key: &str| self.string(asset, key);
        crate::strings::expand(text, &mut look)
    }
}
