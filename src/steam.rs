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

/// Add `launch_cmd` to every Steam app's LaunchOptions.
/// - Games with no LaunchOptions get: `launch_cmd %command%`
/// - Games that already have LaunchOptions but are missing the cmd get it prepended.
/// - Games that already have it are skipped.
pub fn add_launch_option(launch_cmd: &str) -> Result<String, String> {
    let path = find_steam_vdf();
    if !path.exists() {
        return Err("Steam VDF not found".to_string());
    }

    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let app_pat = Regex::new(r#"(?m)^\s*"(\d+)"\s*\{"#).unwrap();
    let launch_pat = Regex::new(r#"("LaunchOptions")\s+"([^"]*)""#).unwrap();

    // Two lists: positions to insert new LaunchOptions, and ranges to replace existing ones
    let mut inserts: Vec<usize> = Vec::new();
    struct Replacement {
        start: usize,
        end: usize,
        new_val: String,
    }
    let mut replacements: Vec<Replacement> = Vec::new();

    for m in app_pat.find_iter(&text) {
        let open_brace = m.end() - 1;

        // Find matching closing brace
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
        let cmd_lower = launch_cmd.to_lowercase();

        if let Some(lm) = launch_pat.find(block) {
            // Block already has LaunchOptions — check if our cmd is already in it
            let cap = launch_pat.captures(block).unwrap();
            let existing = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            if existing.to_lowercase().contains(&cmd_lower) {
                // already there, skip
                continue;
            }
            // Prepend our cmd to the existing value
            let new_val = format!("{launch_cmd} {existing}");
            let abs_start = open_brace + lm.start();
            let abs_end = open_brace + lm.end();
            let full_new = format!("\"LaunchOptions\"\t\t\"{new_val}\"");
            replacements.push(Replacement { start: abs_start, end: abs_end, new_val: full_new });
        } else {
            // No LaunchOptions at all — insert one
            inserts.push(open_brace + 1);
        }
    }

    if inserts.is_empty() && replacements.is_empty() {
        return Ok(format!("{launch_cmd}: already in all Steam games"));
    }

    let added = inserts.len();
    let updated = replacements.len();

    // Apply replacements from back to front to preserve offsets
    let mut out = text.clone();
    let mut all_ops: Vec<(usize, usize, String)> = replacements
        .into_iter()
        .map(|r| (r.start, r.end, r.new_val))
        .collect();
    // Also convert inserts into ops (end == start means insert)
    let launch_line = format!("\n\t\t\t\t\t\"LaunchOptions\"\t\t\"{launch_cmd} %command%\"");
    for pos in inserts {
        all_ops.push((pos, pos, launch_line.clone()));
    }
    all_ops.sort_unstable_by(|a, b| b.0.cmp(&a.0));

    for (start, end, new) in all_ops {
        out.replace_range(start..end, &new);
    }

    std::fs::write(&path, &out).map_err(|e| e.to_string())?;
    crate::config::log_entry(&format!(
        "Steam {launch_cmd}: added to {added} games, updated {updated} games"
    ));
    Ok(format!(
        "{launch_cmd}: lisätty {added} peliin, päivitetty {updated} pelissä"
    ))
}

/// Remove `launch_cmd` token from every Steam game's LaunchOptions.
pub fn remove_launch_option(launch_cmd: &str) -> Result<String, String> {
    let path = find_steam_vdf();
    if !path.exists() {
        return Err("Steam VDF not found".to_string());
    }

    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let launch_pat = Regex::new(r#""LaunchOptions"\s+"([^"]*)""#).unwrap();
    let cmd_lower = launch_cmd.to_lowercase();

    let mut count = 0usize;
    let mut out = text.clone();

    // Collect replacements back-to-front to preserve offsets
    let mut ops: Vec<(usize, usize, String)> = Vec::new();
    for m in launch_pat.find_iter(&text) {
        let cap = launch_pat.captures(&text[m.start()..m.end()]).unwrap();
        let value = cap.get(1).map(|c| c.as_str()).unwrap_or("");
        if !value.to_lowercase().contains(&cmd_lower) {
            continue;
        }
        let new_val: String = value
            .split_whitespace()
            .filter(|t| t.to_lowercase() != cmd_lower)
            .collect::<Vec<_>>()
            .join(" ");
        let new_entry = format!("\"LaunchOptions\"\t\t\"{new_val}\"");
        ops.push((m.start(), m.end(), new_entry));
        count += 1;
    }

    if count == 0 {
        return Ok(format!("{launch_cmd}: ei löydy Steam-peleistä"));
    }

    ops.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    for (start, end, new) in ops {
        out.replace_range(start..end, &new);
    }

    std::fs::write(&path, &out).map_err(|e| e.to_string())?;
    crate::config::log_entry(&format!("Steam {launch_cmd} removed from {count} games"));
    Ok(format!("{launch_cmd}: poistettu {count} pelistä"))
}
