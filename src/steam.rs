use crate::config::home_dir;
use regex::Regex;
use std::path::PathBuf;

pub fn find_steam_vdf() -> PathBuf {
    let userdata = home_dir().join(".local/share/Steam/userdata");
    if let Ok(entries) = std::fs::read_dir(&userdata) {
        for entry in entries.flatten() {
            let vdf = entry.path().join("config/localconfig.vdf");
            if vdf.exists() {
                return vdf;
            }
        }
    }
    userdata.join("unknown/config/localconfig.vdf")
}

pub fn steam_has_gamemode() -> bool {
    let path = find_steam_vdf();
    if !path.exists() {
        return false;
    }
    std::fs::read_to_string(&path)
        .map(|t| t.contains("gamemoderun"))
        .unwrap_or(false)
}

pub fn steam_has_mangohud() -> bool {
    let path = find_steam_vdf();
    if !path.exists() {
        return false;
    }
    std::fs::read_to_string(&path)
        .map(|t| t.to_lowercase().contains("mangohud"))
        .unwrap_or(false)
}

/// Insert `launch_cmd %command%` as LaunchOptions into every Steam app block
/// that doesn't already have a LaunchOptions entry.
pub fn add_launch_option(launch_cmd: &str) -> Result<String, String> {
    let path = find_steam_vdf();
    if !path.exists() {
        return Err("Steam VDF not found".to_string());
    }

    let mut text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let app_pat = Regex::new(r#"(?m)^\s*"(\d+)"\s*\{"#).unwrap();
    let launch_pat = Regex::new(r#""LaunchOptions"\s+"[^"]*""#).unwrap();

    // Collect insertion positions from the original text, largest offset first
    // so that each insert doesn't invalidate subsequent positions.
    let mut insertions: Vec<usize> = Vec::new();

    for m in app_pat.find_iter(&text) {
        let open_brace = m.end() - 1;

        // Walk forward to find the matching closing brace
        let mut depth = 0i32;
        let mut close_brace = open_brace;
        for (i, b) in text[open_brace..].bytes().enumerate() {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        close_brace = open_brace + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let block = &text[open_brace..=close_brace];
        if !launch_pat.is_match(block) {
            insertions.push(open_brace + 1);
        }
    }

    if insertions.is_empty() {
        return Ok(format!("Steam {launch_cmd}: already applied to all games"));
    }

    let count = insertions.len();
    let launch_line = format!("\t\t\t\t\t\"LaunchOptions\"\t\t\"{launch_cmd} %command%\"\n");

    // Insert from largest offset to smallest to preserve earlier positions
    insertions.sort_unstable_by(|a, b| b.cmp(a));
    insertions.dedup();
    for pos in insertions {
        text.insert_str(pos, &launch_line);
    }

    std::fs::write(&path, &text).map_err(|e| e.to_string())?;
    crate::config::log_entry(&format!("Steam {launch_cmd} added to {count} games"));
    Ok(format!("{launch_cmd} added to {count} Steam games"))
}
