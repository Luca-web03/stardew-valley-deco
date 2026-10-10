use std::path::{Path, PathBuf};

use valley_sim::save::SaveData;

pub struct SaveInfo {
    pub path: PathBuf,
    pub farmer: String,
    pub farm: String,
    pub day: String,
    pub money: i32,
}

pub fn save_path(dir: &Path, data: &SaveData) -> PathBuf {
    let clean: String = data.farmer.name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    dir.join(format!("{}_{}.json", if clean.is_empty() { "farmer".into() } else { clean }, data.seed))
}

pub fn write(dir: &Path, data: &SaveData) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let p = save_path(dir, data);
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, data.to_json())?;
    std::fs::rename(&tmp, &p)?;
    Ok(p)
}

pub fn list(dir: &Path) -> Vec<SaveInfo> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(s) = std::fs::read_to_string(&p) else { continue };
        let Ok(d) = SaveData::from_json(&s) else { continue };
        out.push(SaveInfo {
            path: p,
            farmer: d.farmer.name.clone(),
            farm: d.farmer.farm_name.clone(),
            day: format!("{} {}, Year {}", d.clock.season.name(), d.clock.day, d.clock.year),
            money: d.farmer.money,
        });
    }
    out.sort_by(|a, b| a.farmer.cmp(&b.farmer));
    out
}

pub fn load(path: &Path) -> Result<SaveData, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    SaveData::from_json(&s)
}
