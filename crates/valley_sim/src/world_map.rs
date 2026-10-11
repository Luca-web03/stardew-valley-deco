//! The world map (`Data/WorldMap`, public format subset): regions with a
//! base texture and areas; each area has a pixel rectangle, tooltips and
//! world positions linking locations (optionally a tile area) to map
//! pixels, so players can be marked on the map.

use std::collections::BTreeMap;

use serde::Deserialize;
use valley_content::data::Rect;

use crate::world::World;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MapTexture {
    pub id: Option<String>,
    pub texture: String,
    pub source_rect: Option<Rect>,
    pub map_pixel_area: Option<Rect>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MapTooltip {
    pub id: Option<String>,
    pub text: String,
    pub pixel_area: Option<Rect>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MapWorldPosition {
    pub id: Option<String>,
    pub location_name: Option<String>,
    pub location_names: Vec<String>,
    pub tile_area: Option<Rect>,
    pub map_pixel_area: Option<Rect>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MapArea {
    pub id: String,
    pub pixel_area: Rect,
    pub scroll_text: Option<String>,
    pub textures: Vec<MapTexture>,
    pub tooltips: Vec<MapTooltip>,
    pub world_positions: Vec<MapWorldPosition>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MapRegion {
    pub display_name: Option<String>,
    pub base_texture: Vec<MapTexture>,
    pub map_areas: Vec<MapArea>,
}

pub fn load(cm: &mut valley_content::ContentManager) -> BTreeMap<String, MapRegion> {
    cm.load_dict("Data/WorldMap")
}

/// Where something sits on the map: region id and map pixel position.
#[derive(Clone, Debug, PartialEq)]
pub struct MapPoint {
    pub region: String,
    pub x: f32,
    pub y: f32,
}

fn center(r: &Rect) -> (f32, f32) {
    (r.x as f32 + r.width as f32 / 2.0, r.y as f32 + r.height as f32 / 2.0)
}

impl World {
    /// Map position of a tile in a location, if the map knows the place.
    pub fn map_point(&self, location: &str, tile: crate::TilePos) -> Option<MapPoint> {
        for (rid, region) in &self.world_map {
            for area in &region.map_areas {
                for wp in &area.world_positions {
                    let names_match = wp.location_name.as_deref() == Some(location) || wp.location_names.iter().any(|n| n == location);
                    if !names_match || !self.gsq_ok(wp.condition.as_deref()) {
                        continue;
                    }
                    let pixels = wp.map_pixel_area.clone().unwrap_or_else(|| area.pixel_area.clone());
                    let (x, y) = match &wp.tile_area {
                        Some(t) => {
                            if tile.x < t.x || tile.y < t.y || tile.x >= t.x + t.width || tile.y >= t.y + t.height {
                                continue;
                            }
                            let fx = (tile.x - t.x) as f32 / t.width.max(1) as f32;
                            let fy = (tile.y - t.y) as f32 / t.height.max(1) as f32;
                            (pixels.x as f32 + fx * pixels.width as f32, pixels.y as f32 + fy * pixels.height as f32)
                        }
                        None => {
                            // Scale the whole location into the area.
                            let (w, h) = self.locations.get(location).map(|l| (l.width().max(1), l.height().max(1))).unwrap_or((1, 1));
                            if pixels.width > 8 && pixels.height > 8 {
                                (pixels.x as f32 + tile.x as f32 / w as f32 * pixels.width as f32, pixels.y as f32 + tile.y as f32 / h as f32 * pixels.height as f32)
                            } else {
                                center(&pixels)
                            }
                        }
                    };
                    return Some(MapPoint { region: rid.clone(), x, y });
                }
            }
        }
        None
    }

    /// The tooltip under a map pixel (area tooltips, then the area id).
    pub fn map_tooltip(&self, region: &str, x: f32, y: f32) -> Option<String> {
        let r = self.world_map.get(region)?;
        let inside = |p: &Rect| x >= p.x as f32 && y >= p.y as f32 && x < (p.x + p.width) as f32 && y < (p.y + p.height) as f32;
        for area in r.map_areas.iter().filter(|a| self.gsq_ok(a.condition.as_deref())) {
            for t in &area.tooltips {
                let pa = t.pixel_area.clone().unwrap_or_else(|| area.pixel_area.clone());
                if inside(&pa) && self.gsq_ok(t.condition.as_deref()) && !t.text.is_empty() {
                    return Some(self.items.expand_text(&t.text));
                }
            }
        }
        None
    }

    /// The region the farmer is in (else the first region).
    pub fn map_region_here(&self) -> Option<String> {
        self.map_point(&self.current.clone(), self.farmer.tile()).map(|p| p.region).or_else(|| self.world_map.keys().next().cloned())
    }
}

/// Size of a region's map in map pixels (from its base texture).
pub fn map_size(r: &MapRegion) -> (i32, i32) {
    let base = r.base_texture.first();
    let rect = base.and_then(|b| b.map_pixel_area.clone().or(b.source_rect.clone()));
    rect.map(|r| (r.width, r.height)).unwrap_or((300, 180))
}
