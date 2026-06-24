use crate::config::home_dir;
use serde_json::Value;
use std::path::PathBuf;

pub fn find_heroic_cfg() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("heroic/config.json")
}

fn games_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("heroic/GamesConfig")
}

pub fn get_heroic_bool(key: &str) -> bool {
    let path = find_heroic_cfg();
    if !path.exists() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(&path) else { return false; };
    let Ok(data) = serde_json::from_str::<Value>(&text) else { return false; };
    for sec in ["defaultSettings", "settings"] {
        if let Some(val) = data.get(sec).and_then(|s| s.get(key)) {
            return val.as_bool().unwrap_or(false);
        }
    }
    false
}

pub fn toggle_heroic(key: &str, enable: bool) -> Result<String, String> {
    let path = find_heroic_cfg();
    if !path.exists() {
        return Err("Heroic config not found".to_string());
    }

    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut data: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let mut updated = false;
    for sec in ["defaultSettings", "settings"] {
        if let Some(obj) = data.get_mut(sec).and_then(|v| v.as_object_mut()) {
            obj.insert(key.to_string(), Value::Bool(enable));
            updated = true;
            break;
        }
    }
    if !updated {
        if let Some(obj) = data.as_object_mut() {
            obj.insert(key.to_string(), Value::Bool(enable));
        }
    }

    std::fs::write(&path, serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // Päivitä pelikohtaiset konfit GamesConfig/*.json
    let mut game_count = 0usize;
    let gdir = games_dir();
    if gdir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&gdir) {
            for entry in entries.flatten() {
                let fpath = entry.path();
                if fpath.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                // Pelin UUID = tiedostonimi ilman päätettä
                let app_name = match fpath.file_stem().and_then(|s| s.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let Ok(gtext) = std::fs::read_to_string(&fpath) else { continue; };
                let Ok(mut gdata) = serde_json::from_str::<Value>(&gtext) else { continue; };

                // Kohdista VAIN pelin UUID-avaimeen (ei "explicit" tai "version")
                let mut changed = false;
                if let Some(cfg_obj) = gdata.get_mut(&app_name).and_then(|v| v.as_object_mut()) {
                    // Heroic käyttää kahta eri nimeä MangoHudille — päivitä molemmat
                    let keys: Vec<&str> = if key == "enableMangoHud" || key == "showMangohud" {
                        vec!["enableMangoHud", "showMangohud"]
                    } else {
                        vec![key.as_ref()]
                    };
                    for k in &keys {
                        let current = cfg_obj.get(*k).and_then(|v| v.as_bool());
                        if current != Some(enable) {
                            cfg_obj.insert((*k).to_string(), Value::Bool(enable));
                            changed = true;
                        }
                    }
                }

                if changed {
                    if let Ok(new_text) = serde_json::to_string_pretty(&gdata) {
                        let _ = std::fs::write(&fpath, new_text);
                        game_count += 1;
                    }
                }
            }
        }
    }

    crate::config::log_entry(&format!(
        "Heroic {key} -> {enable}, updated {game_count} game configs"
    ));
    Ok(format!(
        "{key} {} — {} pelikohtaista konffitiedostoa päivitetty",
        if enable { "käytössä" } else { "pois käytöstä" },
        game_count
    ))
}
