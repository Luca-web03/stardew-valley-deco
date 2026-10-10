//! Content mods: folders under a mods directory that mirror the Content
//! layout. Non-JSON files (images, maps, audio) replace the originals; JSON
//! assets are merged into them so a mod can add or change single entries:
//!
//! * objects: keys are added or replaced; a `null` value removes the key
//! * arrays: entries with an `Id` replace the entry with the same `Id`
//!   (or are appended); other entries are appended
//!
//! An optional `manifest.json` gives `Name`, `Version`, `Author`,
//! `Description` and `Priority` (higher loads later and wins). Mods load in
//! (priority, folder name) order. A mod whose manifest sets `"Enabled": false`
//! is skipped.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
struct Manifest {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
    description: Option<String>,
    priority: i32,
    enabled: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub priority: i32,
    pub dir: PathBuf,
}

/// Find the mods in `mods_dir`, in load order.
pub fn discover(mods_dir: &Path) -> Vec<ModInfo> {
    let Ok(entries) = std::fs::read_dir(mods_dir) else { return vec![] };
    let mut mods: Vec<ModInfo> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let dir = e.path();
            let folder = e.file_name().to_string_lossy().to_string();
            if folder.starts_with('.') {
                return None;
            }
            let manifest: Manifest = std::fs::read(dir.join("manifest.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&b)).ok())
                .unwrap_or_default();
            if manifest.enabled == Some(false) {
                return None;
            }
            Some(ModInfo {
                name: manifest.name.unwrap_or(folder),
                version: manifest.version.unwrap_or_default(),
                author: manifest.author.unwrap_or_default(),
                description: manifest.description.unwrap_or_default(),
                priority: manifest.priority,
                dir,
            })
        })
        .collect();
    mods.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.dir.file_name().cmp(&b.dir.file_name())));
    mods
}

/// Merge `patch` into `base` following the rules in the module docs.
pub fn merge_json(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    b.remove(k);
                } else {
                    b.insert(k.clone(), v.clone());
                }
            }
        }
        (Value::Array(b), Value::Array(p)) => {
            for v in p {
                let id = v.get("Id").and_then(Value::as_str);
                match id.and_then(|id| b.iter().position(|e| e.get("Id").and_then(Value::as_str) == Some(id))) {
                    Some(i) => b[i] = v.clone(),
                    None => b.push(v.clone()),
                }
            }
        }
        (b, p) => *b = p.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn objects_merge_by_key_and_null_removes() {
        let mut base = json!({ "a": 1, "b": { "x": 1 }, "c": 3 });
        merge_json(&mut base, &json!({ "b": { "y": 2 }, "c": null, "d": 4 }));
        assert_eq!(base, json!({ "a": 1, "b": { "y": 2 }, "d": 4 }));
    }

    #[test]
    fn arrays_merge_by_id() {
        let mut base = json!([{ "Id": "a", "v": 1 }, { "Id": "b", "v": 2 }]);
        merge_json(&mut base, &json!([{ "Id": "b", "v": 20 }, { "Id": "c", "v": 3 }, 5]));
        assert_eq!(base, json!([{ "Id": "a", "v": 1 }, { "Id": "b", "v": 20 }, { "Id": "c", "v": 3 }, 5]));
    }

    #[test]
    fn discovery_orders_by_priority_and_skips_disabled() {
        let root = std::env::temp_dir().join(format!("valley_mods_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (dir, manifest) in [("b_mod", r#"{"Name":"Bee","Priority":0}"#), ("a_mod", r#"{"Priority":5}"#), ("c_mod", r#"{"Enabled":false}"#), ("d_mod", "")] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            if !manifest.is_empty() {
                std::fs::write(root.join(dir).join("manifest.json"), manifest).unwrap();
            }
        }
        let names: Vec<String> = discover(&root).into_iter().map(|m| m.name).collect();
        assert_eq!(names, vec!["Bee", "d_mod", "a_mod"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
