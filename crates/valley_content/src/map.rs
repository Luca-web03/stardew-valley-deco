use crate::props::{PropValue, Props, PropsExt};

/// A tilesheet referenced by a map. `image` is the asset name of the texture
/// (relative to the map's folder, without extension), e.g. "spring_outdoorsTileSheet".
#[derive(Clone, Debug, Default)]
pub struct TileSheet {
    pub id: String,
    pub image: String,
    /// Sheet size in tiles.
    pub sheet_w: i32,
    pub sheet_h: i32,
    pub tile_w: i32,
    pub tile_h: i32,
    pub margin: (i32, i32),
    pub spacing: (i32, i32),
    pub props: Props,
}

impl TileSheet {
    /// Per-tile-index properties stored on the sheet ("@TileIndex@<n>@<Name>").
    pub fn tile_prop(&self, index: i32, name: &str) -> Option<&PropValue> {
        self.props.get(&format!("@TileIndex@{index}@{name}"))
    }

    pub fn set_tile_prop(&mut self, index: i32, name: &str, v: PropValue) {
        self.props.insert(format!("@TileIndex@{index}@{name}"), v);
    }

    /// Source rectangle (x, y, w, h) of a tile index in the sheet texture.
    pub fn src(&self, index: i32) -> (i32, i32, i32, i32) {
        let cols = self.sheet_w.max(1);
        let col = index % cols;
        let row = index / cols;
        (
            self.margin.0 + col * (self.tile_w + self.spacing.0),
            self.margin.1 + row * (self.tile_h + self.spacing.1),
            self.tile_w,
            self.tile_h,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimatedTile {
    /// Frames as (sheet index, tile index).
    pub frames: Vec<(u16, i32)>,
    pub interval_ms: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub sheet: u16,
    pub index: i32,
    pub props: Props,
    pub anim: Option<AnimatedTile>,
}

impl Tile {
    pub fn new(sheet: u16, index: i32) -> Self {
        Self { sheet, index, props: Props::new(), anim: None }
    }

    /// The (sheet, index) to show at `ms` milliseconds of game time.
    pub fn frame_at(&self, ms: u64) -> (u16, i32) {
        match &self.anim {
            Some(a) if !a.frames.is_empty() && a.interval_ms > 0 => {
                let n = a.frames.len() as u64;
                a.frames[((ms / a.interval_ms as u64) % n) as usize]
            }
            _ => (self.sheet, self.index),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Layer {
    pub id: String,
    pub visible: bool,
    pub width: i32,
    pub height: i32,
    pub props: Props,
    pub tiles: Vec<Option<Tile>>,
}

impl Layer {
    pub fn new(id: &str, width: i32, height: i32) -> Self {
        Self {
            id: id.to_string(),
            visible: true,
            width,
            height,
            props: Props::new(),
            tiles: vec![None; (width * height).max(0) as usize],
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<&Tile> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        self.tiles[(y * self.width + x) as usize].as_ref()
    }

    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut Tile> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        self.tiles[(y * self.width + x) as usize].as_mut()
    }

    pub fn set(&mut self, x: i32, y: i32, t: Option<Tile>) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }
        self.tiles[(y * self.width + x) as usize] = t;
    }
}

#[derive(Clone, Debug, Default)]
pub struct Map {
    pub id: String,
    pub width: i32,
    pub height: i32,
    pub tile_w: i32,
    pub tile_h: i32,
    pub props: Props,
    pub sheets: Vec<TileSheet>,
    pub layers: Vec<Layer>,
}

impl Map {
    pub fn layer(&self, id: &str) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id.eq_ignore_ascii_case(id))
    }

    pub fn layer_mut(&mut self, id: &str) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id.eq_ignore_ascii_case(id))
    }

    pub fn sheet_index(&self, id: &str) -> Option<u16> {
        self.sheets.iter().position(|s| s.id == id).map(|i| i as u16)
    }

    pub fn tile(&self, layer: &str, x: i32, y: i32) -> Option<&Tile> {
        self.layer(layer).and_then(|l| l.get(x, y))
    }

    /// Look up a tile property: first on the tile instance, then on the
    /// tilesheet's per-index properties (same precedence as the original).
    pub fn tile_prop(&self, layer: &str, x: i32, y: i32, name: &str) -> Option<String> {
        let t = self.tile(layer, x, y)?;
        if let Some(v) = t.props.get(name) {
            return Some(v.as_string());
        }
        self.sheets.get(t.sheet as usize)?.tile_prop(t.index, name).map(|v| v.as_string())
    }

    pub fn prop(&self, name: &str) -> Option<String> {
        self.props.str(name)
    }
}
