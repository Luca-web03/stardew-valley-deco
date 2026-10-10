use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::map::Map;
use crate::source::{normalize, ContentSource, DirSource, LayeredSource, ReadSeek};
use crate::xnb::{self, XnbAsset};

/// Loads and caches assets by game asset name (e.g. `Maps/Farm`,
/// `Data/Objects`, `TileSheets/crops`).
pub struct ContentManager {
    source: Arc<dyn ContentSource>,
    /// Game content + placeholder pack, without mods (base for JSON merges).
    base: Option<Arc<dyn ContentSource>>,
    /// Loaded mods in load order, each with its own folder source.
    mod_sources: Vec<(crate::mods::ModInfo, Arc<DirSource>)>,
    json_cache: HashMap<String, Arc<serde_json::Value>>,
    string_cache: HashMap<String, HashMap<String, String>>,
    pub warnings: Vec<String>,
    /// Language suffix for localized assets, e.g. "de-DE". Empty = default.
    pub locale: String,
    /// True if any user-provided (real game) content directory is mounted.
    pub has_game_content: bool,
    /// The user content directory, if one was given.
    pub game_dir: Option<PathBuf>,
}

impl ContentManager {
    pub fn new(source: Box<dyn ContentSource>) -> Self {
        Self {
            source: Arc::from(source),
            base: None,
            mod_sources: Vec::new(),
            json_cache: HashMap::new(),
            string_cache: HashMap::new(),
            warnings: Vec::new(),
            locale: String::new(),
            has_game_content: false,
            game_dir: None,
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
        cm.game_dir = dir.map(|d| d.to_path_buf());
        cm
    }

    /// Game content and placeholder pack with the mods found in `mods_dir`
    /// on top (see [`crate::mods`]).
    pub fn with_mods(dir: Option<&std::path::Path>, mods_dir: Option<&std::path::Path>) -> Self {
        let mut cm = Self::with_game_dir(dir);
        let mods = mods_dir.map(crate::mods::discover).unwrap_or_default();
        if mods.is_empty() {
            return cm;
        }
        let base = cm.source.clone();
        let mut layered = LayeredSource::default();
        // Later mods win: highest in load order is consulted first.
        for m in mods.iter().rev() {
            layered.push(Box::new(DirSource::new(&m.dir)));
        }
        layered.push(Box::new(SharedSource(base.clone())));
        cm.mod_sources = mods.into_iter().map(|m| {
            let src = Arc::new(DirSource::new(&m.dir));
            (m, src)
        }).collect();
        cm.base = Some(base);
        cm.source = Arc::new(layered);
        cm
    }

    /// Loaded mods, in load order.
    pub fn mods(&self) -> Vec<&crate::mods::ModInfo> {
        self.mod_sources.iter().map(|(m, _)| m).collect()
    }

    /// Placeholder pack only.
    pub fn dev_only() -> Self {
        Self::with_game_dir(None)
    }

    pub fn describe(&self) -> String {
        self.source.describe()
    }

    /// Raw bytes of a content file by relative path *including* extension
    /// (e.g. `XACT/Sound Bank.xsb`), through the same layered source as
    /// every other asset.
    pub fn read_raw(&self, path: &str) -> Option<Vec<u8>> {
        self.source.read(&normalize(path))
    }

    /// Open a content file for incremental (seekable) reads; used for large
    /// files such as audio wave banks that should not be loaded whole.
    pub fn open_raw(&self, path: &str) -> Option<Box<dyn ReadSeek>> {
        self.source.open(&normalize(path))
    }

    /// Shared handle to the underlying source, for subsystems that resolve
    /// files later or from another thread (e.g. audio).
    pub fn raw_source(&self) -> Arc<dyn ContentSource> {
        self.source.clone()
    }

    fn read_with_ext(&self, asset: &str, exts: &[&str]) -> Option<(String, Vec<u8>)> {
        Self::read_from(&*self.source, &self.locale, asset, exts)
    }

    fn read_from(source: &dyn ContentSource, locale: &str, asset: &str, exts: &[&str]) -> Option<(String, Vec<u8>)> {
        let base = normalize(asset);
        for ext in exts {
            if !locale.is_empty() {
                let p = format!("{base}.{locale}.{ext}");
                if let Some(b) = source.read(&p) {
                    return Some((ext.to_string(), b));
                }
            }
            let p = format!("{base}.{ext}");
            if let Some(b) = source.read(&p) {
                return Some((ext.to_string(), b));
            }
        }
        None
    }

    fn parse_json(&mut self, asset: &str, bytes: &[u8]) -> Option<serde_json::Value> {
        // Tolerate a UTF-8 BOM.
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
        match serde_json::from_slice::<serde_json::Value>(bytes) {
            Ok(v) => Some(v),
            Err(e) => {
                self.warnings.push(format!("{asset}: {e}"));
                None
            }
        }
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
        let value = if let Some(base) = self.base.clone() {
            // Base content first, then each mod merged in load order.
            let mut value = match Self::read_from(&*base, &self.locale, asset, &["json"]) {
                Some((_, b)) => self.parse_json(asset, &b),
                None => None,
            };
            let mods: Vec<_> = self.mod_sources.iter().map(|(m, s)| (m.name.clone(), s.clone())).collect();
            for (name, src) in mods {
                let Some((_, b)) = Self::read_from(&*src, &self.locale, asset, &["json"]) else { continue };
                let Some(patch) = self.parse_json(&format!("{name}: {asset}"), &b) else { continue };
                match value.as_mut() {
                    Some(v) => crate::mods::merge_json(v, &patch),
                    None => value = Some(patch),
                }
            }
            value?
        } else {
            let (_, bytes) = self.read_with_ext(asset, &["json"])?;
            self.parse_json(asset, &bytes)?
        };
        let v = Arc::new(value);
        self.json_cache.insert(key, v.clone());
        Some(v)
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

/// Lets an already shared source sit inside a [`LayeredSource`].
struct SharedSource(Arc<dyn ContentSource>);

impl ContentSource for SharedSource {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0.read(path)
    }
    fn open(&self, path: &str) -> Option<Box<dyn ReadSeek>> {
        self.0.open(path)
    }
    fn describe(&self) -> String {
        self.0.describe()
    }
}
