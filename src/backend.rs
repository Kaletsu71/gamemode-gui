use crate::{config, heroic, steam};
use egui::Context;
use std::sync::mpsc::Sender;

pub enum BackendMsg {
    GameModeStatus(String),
    MangoHudInstalled(bool),
    HeroicGmStatus(bool),
    HeroicMhStatus(bool),
    SteamGmStatus(bool),
    SteamMhStatus(bool),
    Distro(String),
    StatusDone,
    OperationDone(String),
    Error(String),
}

// ── helpers ────────────────────────────────────────────────────

pub fn check_gamemode_status() -> String {
    match std::process::Command::new("gamemoded")
        .arg("-s")
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if stdout.contains("active") || stdout.contains("on") {
                "ON".to_string()
            } else {
                let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if raw.is_empty() { "OFF".to_string() } else { raw }
            }
        }
        Err(_) => "Not installed".to_string(),
    }
}

fn mangohud_in_path() -> bool {
    std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|dir| std::path::Path::new(dir).join("mangohud").exists())
}

fn distro_name() -> String {
    if let Ok(text) = std::fs::read_to_string("/etc/os-release") {
        for line in text.lines() {
            if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                return val.trim_matches('"').to_string();
            }
        }
    }
    "Unknown".to_string()
}

// ── spawners ───────────────────────────────────────────────────

pub fn spawn_status_check(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        tx.send(BackendMsg::GameModeStatus(check_gamemode_status())).ok();
        tx.send(BackendMsg::MangoHudInstalled(mangohud_in_path())).ok();
        tx.send(BackendMsg::HeroicGmStatus(heroic::get_heroic_bool("useGameMode"))).ok();
        tx.send(BackendMsg::HeroicMhStatus(heroic::get_heroic_bool("enableMangoHud"))).ok();
        tx.send(BackendMsg::SteamGmStatus(steam::steam_has_gamemode())).ok();
        tx.send(BackendMsg::SteamMhStatus(steam::steam_has_mangohud())).ok();
        tx.send(BackendMsg::Distro(distro_name())).ok();
        tx.send(BackendMsg::StatusDone).ok();
        config::log_entry("Status check done");
        ctx.request_repaint();
    });
}

pub fn spawn_steam_gamemode(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        let msg = match steam::add_launch_option("gamemoderun") {
            Ok(m) => BackendMsg::OperationDone(m),
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}

pub fn spawn_steam_mangohud(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        let msg = match steam::add_launch_option("mangohud") {
            Ok(m) => BackendMsg::OperationDone(m),
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}

pub fn spawn_heroic_toggle(key: String, enable: bool, tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        let msg = match heroic::toggle_heroic(&key, enable) {
            Ok(m) => BackendMsg::OperationDone(m),
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}
