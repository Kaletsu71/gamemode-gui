// AI VRAM hook: install/remove GameMode custom start/end scripts that stop
// local llama.cpp model services while a game runs, freeing VRAM, and start
// them again afterwards. The hook scripts are embedded from `scripts/` so the
// app is self-contained and writes the canonical copies on install.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

// Embedded hook scripts (the canonical versions live in the repo `scripts/`).
const START_SCRIPT: &str = include_str!("../scripts/llama-game-start");
const END_SCRIPT: &str = include_str!("../scripts/llama-game-end");

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/root"))
}

fn start_script_path() -> PathBuf {
    home().join(".local/bin/llama-game-start")
}

fn end_script_path() -> PathBuf {
    home().join(".local/bin/llama-game-end")
}

fn gamemode_ini_path() -> PathBuf {
    home().join(".config/gamemode.ini")
}

/// True when the hook is wired up: the gamemode.ini `[custom]` start line points
/// at our script and both script files exist.
pub fn is_installed() -> bool {
    let start = start_script_path();
    let end = end_script_path();
    if !start.exists() || !end.exists() {
        return false;
    }
    let start_str = start.to_string_lossy().to_string();
    match std::fs::read_to_string(gamemode_ini_path()) {
        Ok(ini) => ini.lines().any(|l| {
            let t = l.trim();
            t.starts_with("start=") && t.contains(&start_str)
        }),
        Err(_) => false,
    }
}

/// Write the two hook scripts (executable) and merge the `[custom]` section into
/// gamemode.ini, preserving any other sections the user already has.
pub fn install() -> Result<String, String> {
    let bin_dir = home().join(".local/bin");
    std::fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;

    write_executable(&start_script_path(), START_SCRIPT)?;
    write_executable(&end_script_path(), END_SCRIPT)?;

    let ini = gamemode_ini_path();
    if let Some(parent) = ini.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let existing = std::fs::read_to_string(&ini).unwrap_or_default();
    let start_str = start_script_path().to_string_lossy().to_string();
    let end_str = end_script_path().to_string_lossy().to_string();
    let merged = upsert_custom(&existing, &start_str, &end_str);
    std::fs::write(&ini, merged).map_err(|e| e.to_string())?;

    Ok("AI VRAM hook installed → models stop while gaming".to_string())
}

/// Remove our hook lines from gamemode.ini and delete the script files.
pub fn remove() -> Result<String, String> {
    let ini = gamemode_ini_path();
    if let Ok(existing) = std::fs::read_to_string(&ini) {
        let cleaned = strip_custom(&existing);
        std::fs::write(&ini, cleaned).map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_file(start_script_path());
    let _ = std::fs::remove_file(end_script_path());
    Ok("AI VRAM hook removed".to_string())
}

fn write_executable(path: &PathBuf, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| e.to_string())?;
    let mut perm = std::fs::metadata(path).map_err(|e| e.to_string())?.permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(path, perm).map_err(|e| e.to_string())?;
    Ok(())
}

/// Insert or update `start=`/`end=` inside the `[custom]` section, leaving every
/// other line untouched. Appends a fresh `[custom]` section when none exists.
fn upsert_custom(content: &str, start_path: &str, end_path: &str) -> String {
    let start_line = format!("start={start_path}");
    let end_line = format!("end={end_path}");

    if !content.contains("[custom]") {
        let mut out = content.trim_end().to_string();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(&format!("[custom]\n{start_line}\n{end_line}\n"));
        return out;
    }

    let mut out: Vec<String> = Vec::new();
    let mut in_custom = false;
    let mut wrote_start = false;
    let mut wrote_end = false;

    for line in content.lines() {
        let trimmed = line.trim();
        let is_header = trimmed.starts_with('[') && trimmed.ends_with(']');
        if is_header {
            if in_custom {
                if !wrote_start {
                    out.push(start_line.clone());
                }
                if !wrote_end {
                    out.push(end_line.clone());
                }
            }
            in_custom = trimmed == "[custom]";
            wrote_start = false;
            wrote_end = false;
            out.push(line.to_string());
            continue;
        }
        if in_custom {
            let key = trimmed
                .trim_start_matches(';')
                .trim_start_matches('#')
                .trim_start();
            if key.starts_with("start=") {
                out.push(start_line.clone());
                wrote_start = true;
                continue;
            }
            if key.starts_with("end=") {
                out.push(end_line.clone());
                wrote_end = true;
                continue;
            }
        }
        out.push(line.to_string());
    }
    if in_custom {
        if !wrote_start {
            out.push(start_line.clone());
        }
        if !wrote_end {
            out.push(end_line.clone());
        }
    }

    let mut s = out.join("\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Drop our `start=`/`end=` lines (those referencing the llama hook scripts) and
/// remove the `[custom]` header if nothing else remains in that section.
fn strip_custom(content: &str) -> String {
    // First pass: drop lines that reference our hook scripts.
    let kept: Vec<&str> = content
        .lines()
        .filter(|l| !(l.contains("llama-game-start") || l.contains("llama-game-end")))
        .collect();

    // Second pass: remove a now-empty `[custom]` header.
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < kept.len() {
        let line = kept[i];
        if line.trim() == "[custom]" {
            // Peek ahead: is there any key=value before the next header / EOF?
            let mut j = i + 1;
            let mut has_body = false;
            while j < kept.len() {
                let t = kept[j].trim();
                if t.starts_with('[') && t.ends_with(']') {
                    break;
                }
                if !t.is_empty() && !t.starts_with(';') && !t.starts_with('#') {
                    has_body = true;
                    break;
                }
                j += 1;
            }
            if !has_body {
                // skip the empty header (and trailing blank lines it owned)
                i += 1;
                while i < kept.len() && kept[i].trim().is_empty() {
                    i += 1;
                }
                continue;
            }
        }
        out.push(line.to_string());
        i += 1;
    }

    let mut s = out.join("\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}
