use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Normalize an asset path: backslashes to '/', strip leading "./" and "Content/".
pub fn normalize(path: &str) -> String {
    let mut p = path.replace('\\', "/");
    while let Some(rest) = p.strip_prefix("./") {
        p = rest.to_string();
    }
    if p.len() >= 8 && p[..8].eq_ignore_ascii_case("content/") {
        p = p[8..].to_string();
    }
    p
}

/// A readable, seekable byte stream (an open content file).
pub trait ReadSeek: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeek for T {}

/// Something that can produce the raw bytes of a content file.
pub trait ContentSource: Send + Sync {
    /// `path` is a normalized relative path including extension.
    fn read(&self, path: &str) -> Option<Vec<u8>>;
    /// Open a file for incremental reads, for large files such as audio
    /// wave banks. The default loads the whole file into memory.
    fn open(&self, path: &str) -> Option<Box<dyn ReadSeek>> {
        self.read(path).map(|b| Box::new(Cursor::new(b)) as Box<dyn ReadSeek>)
    }
    fn describe(&self) -> String;
}

/// Shared sources (e.g. a [`crate::ContentManager`]'s source reused by audio).
impl<T: ContentSource + ?Sized> ContentSource for std::sync::Arc<T> {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        (**self).read(path)
    }
    fn open(&self, path: &str) -> Option<Box<dyn ReadSeek>> {
        (**self).open(path)
    }
    fn describe(&self) -> String {
        (**self).describe()
    }
}

/// A directory on disk. Lookups are case-insensitive so data written for
/// Windows paths resolves on case-sensitive filesystems.
pub struct DirSource {
    root: PathBuf,
    index: Mutex<Option<HashMap<String, PathBuf>>>,
}

impl DirSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into(), index: Mutex::new(None) }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn build_index(root: &Path) -> HashMap<String, PathBuf> {
        let mut map = HashMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(rel) = p.strip_prefix(root) {
                    let key = rel.to_string_lossy().replace('\\', "/").to_lowercase();
                    map.insert(key, p);
                }
            }
        }
        map
    }
}

impl DirSource {
    /// Resolve a relative path to a file on disk (case-insensitively).
    pub fn locate(&self, path: &str) -> Option<PathBuf> {
        let direct = self.root.join(path);
        if direct.is_file() {
            return Some(direct);
        }
        let mut idx = self.index.lock().ok()?;
        let map = idx.get_or_insert_with(|| Self::build_index(&self.root));
        map.get(&path.to_lowercase()).cloned()
    }
}

impl ContentSource for DirSource {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        std::fs::read(self.locate(path)?).ok()
    }

    fn open(&self, path: &str) -> Option<Box<dyn ReadSeek>> {
        let f = std::fs::File::open(self.locate(path)?).ok()?;
        Some(Box::new(std::io::BufReader::new(f)))
    }

    fn describe(&self) -> String {
        format!("dir:{}", self.root.display())
    }
}

/// In-memory files (used by the built-in placeholder pack and tests).
#[derive(Default)]
pub struct MemSource {
    pub name: String,
    files: HashMap<String, Vec<u8>>,
}

impl MemSource {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), files: HashMap::new() }
    }
    pub fn insert(&mut self, path: &str, bytes: Vec<u8>) {
        self.files.insert(normalize(path).to_lowercase(), bytes);
    }
    pub fn insert_str(&mut self, path: &str, s: &str) {
        self.insert(path, s.as_bytes().to_vec());
    }
    pub fn paths(&self) -> impl Iterator<Item = &String> {
        self.files.keys()
    }
}

impl ContentSource for MemSource {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.files.get(&path.to_lowercase()).cloned()
    }
    fn describe(&self) -> String {
        format!("mem:{}", self.name)
    }
}

/// Tries sources in order; the first hit wins (user content over fallback).
#[derive(Default)]
pub struct LayeredSource {
    pub layers: Vec<Box<dyn ContentSource>>,
}

impl LayeredSource {
    pub fn push(&mut self, s: Box<dyn ContentSource>) {
        self.layers.push(s);
    }
}

impl ContentSource for LayeredSource {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.layers.iter().find_map(|l| l.read(path))
    }
    fn open(&self, path: &str) -> Option<Box<dyn ReadSeek>> {
        self.layers.iter().find_map(|l| l.open(path))
    }
    fn describe(&self) -> String {
        self.layers.iter().map(|l| l.describe()).collect::<Vec<_>>().join(" > ")
    }
}
