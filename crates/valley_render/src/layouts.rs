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
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layouts {
    pub character: CharacterLayout,
    pub crops: CropLayout,
    pub hoe_dirt: HoeDirtLayout,
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
