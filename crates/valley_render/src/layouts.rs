//! Sprite-sheet layout assumptions, overridable with `valley_layouts.json`
//! at the root of the content folder. Defaults match the placeholder pack.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct CharacterLayout {
    pub frame_w: i32,
    pub frame_h: i32,
    /// Sheet rows for facing down, right, up, left.
    pub rows: [i32; 4],
    pub walk_frames: i32,
}

impl Default for CharacterLayout {
    fn default() -> Self {
        Self { frame_w: 16, frame_h: 32, rows: [0, 1, 2, 3], walk_frames: 4 }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct CropLayout {
    pub frame_w: i32,
    pub frame_h: i32,
    pub frames_per_crop: i32,
    pub crops_per_row: i32,
}

impl Default for CropLayout {
    fn default() -> Self {
        Self { frame_w: 16, frame_h: 32, frames_per_crop: 8, crops_per_row: 2 }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct HoeDirtLayout {
    pub texture: String,
    /// Connection variants are laid out in this many columns.
    pub cols: i32,
    pub watered_offset: [i32; 2],
}

impl Default for HoeDirtLayout {
    fn default() -> Self {
        Self { texture: "TerrainFeatures/hoeDirt".into(), cols: 4, watered_offset: [64, 0] }
    }
}

/// Fence sheets: 16x32 frames for each neighbour mask (N=1, E=2, S=4,
/// W=8) in `cols` columns, then gate frames.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct FenceLayout {
    pub cols: i32,
    pub frame_w: i32,
    pub frame_h: i32,
    pub gate_closed: i32,
    pub gate_open: i32,
}

impl Default for FenceLayout {
    fn default() -> Self {
        Self { cols: 4, frame_w: 16, frame_h: 32, gate_closed: 16, gate_open: 17 }
    }
}

/// Floors and paths: from each entry's `Corner`, 16x16 tiles for each
/// neighbour mask in `cols` columns.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct FlooringLayout {
    pub cols: i32,
    pub tile: i32,
}

impl Default for FlooringLayout {
    fn default() -> Self {
        Self { cols: 4, tile: 16 }
    }
}

/// Bush sheet: source rects (x, y, w, h) per size, drawn bottom-left on
/// the bush's tile; `ready_offset` moves to the frame with berries/leaves.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct BushLayout {
    pub texture: String,
    pub small: [i32; 4],
    pub medium: [i32; 4],
    pub large: [i32; 4],
    pub tea_young: [i32; 4],
    pub tea: [i32; 4],
    pub ready_offset: [i32; 2],
}

impl Default for BushLayout {
    fn default() -> Self {
        Self {
            texture: "TileSheets/Dev_Bushes".into(),
            small: [0, 0, 16, 32],
            medium: [16, 0, 32, 32],
            large: [48, 0, 48, 32],
            tea_young: [96, 0, 16, 32],
            tea: [112, 0, 16, 32],
            ready_offset: [0, 32],
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct TreeLayout {
    /// `{kind}` and `{season}` are substituted.
    pub texture: String,
    /// Source rect (x, y, w, h) of a grown tree, drawn bottom-centered on its tile.
    pub canopy: [i32; 4],
    pub stump: [i32; 4],
}

impl Default for TreeLayout {
    fn default() -> Self {
        Self { texture: "TerrainFeatures/dev_tree_{season}".into(), canopy: [48, 0, 48, 96], stump: [0, 48, 16, 16] }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct FarmerLayout {
    /// Use this sheet (laid out like `character`) instead of the generated farmer.
    pub texture: Option<String>,
    /// Compose the farmer from these layers, bottom first (e.g. body, pants,
    /// shirt, hair from the game's own sheets). Takes precedence over
    /// `texture` when non-empty.
    pub layers: Vec<FarmerLayer>,
}

/// One layer of a composed farmer. The output is the standard 4x4 sheet of
/// 16x32 frames (rows down/right/up/left, columns = walk frames); for each
/// output frame the layer copies its own source cell at
/// (`origin` + col * frame_w, `origin` + row * frame_h) for the matching
/// facing row and walk column, places it at `offset` inside the frame, and
/// multiplies it by `tint`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct FarmerLayer {
    pub texture: String,
    pub frame_w: i32,
    pub frame_h: i32,
    /// Source row for facing down, right, up, left.
    pub rows: [i32; 4],
    /// Source column for walk frames 0..3.
    pub cols: [i32; 4],
    /// Top-left of this layer's block in the source sheet (pixels).
    pub origin: [i32; 2],
    /// Where the cell goes inside the 16x32 output frame (pixels).
    pub offset: [i32; 2],
    /// Multiply by an appearance colour: "skin", "hair", "shirt", "pants",
    /// or none.
    pub tint: Option<String>,
    /// Mirror the cell (e.g. reuse right-facing art for left).
    pub flip_left: bool,
}

impl Default for FarmerLayer {
    fn default() -> Self {
        Self { texture: String::new(), frame_w: 16, frame_h: 32, rows: [0, 1, 2, 3], cols: [0, 1, 2, 3], origin: [0, 0], offset: [0, 0], tint: None, flip_left: false }
    }
}

/// Worn hats: each hat is one column of four cells (one per facing) in a
/// band; hats fill the sheet left to right, then the next band.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct HatLayout {
    /// Sheet used when a hat's data names no texture.
    pub texture: String,
    pub cell_w: i32,
    pub cell_h: i32,
    /// Row within a hat's band for facing down, right, up, left.
    pub facing_rows: [i32; 4],
    /// Where the cell's top-left goes relative to the farmer sprite's
    /// top-left, in art pixels.
    pub offset: [i32; 2],
}

impl Default for HatLayout {
    fn default() -> Self {
        Self { texture: "Characters/Farmer/Dev_Hats".into(), cell_w: 16, cell_h: 16, facing_rows: [0, 1, 2, 3], offset: [0, -3] }
    }
}

/// Icons for clothing items: a 16x16 grid on this sheet unless the item's
/// data names its own texture.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct ClothingIconLayout {
    pub texture: String,
}

impl Default for ClothingIconLayout {
    fn default() -> Self {
        Self { texture: "TileSheets/Dev_Clothes".into() }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layouts {
    pub hats: HatLayout,
    pub clothing_icons: ClothingIconLayout,
    pub character: CharacterLayout,
    pub crops: CropLayout,
    pub hoe_dirt: HoeDirtLayout,
    pub fences: FenceLayout,
    pub flooring: FlooringLayout,
    pub bushes: BushLayout,
    pub trees: TreeLayout,
    pub farmer: FarmerLayout,
}

impl Layouts {
    pub fn load(cm: &mut valley_content::ContentManager) -> Self {
        cm.load_json::<Layouts>("valley_layouts").unwrap_or_default()
    }

    pub fn tree_texture(&self, kind: &str, season: &str) -> String {
        self.trees.texture.replace("{kind}", kind).replace("{season}", season)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_override_keeps_defaults() {
        let l: Layouts = serde_json::from_str(r#"{ "trees": { "texture": "T/tree{kind}_{season}" }, "crops": { "crops_per_row": 3 } }"#).unwrap();
        assert_eq!(l.tree_texture("1", "fall"), "T/tree1_fall");
        assert_eq!(l.trees.canopy, [48, 0, 48, 96]);
        assert_eq!(l.crops.crops_per_row, 3);
        assert_eq!(l.crops.frames_per_crop, 8);
        assert_eq!(l.character, CharacterLayout::default());
    }
}
