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
    // gamemoded -s kysyy daemonilta tilan ilman että se käynnistää daemonia
    match std::process::Command::new("gamemoded").arg("-s").output() {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            config::log_entry(&format!("gamemoded -s: {}", text.trim()));
            // "gamemode is active" → ON, "gamemode is inactive" → OFF
            if text.contains("is active") { "ON".to_string() } else { "OFF".to_string() }
        }
        // Binääriä ei löydy → ei asenettu
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

// Sulje Steam siististi -shutdown-signaalilla, odota sammuminen
fn kill_steam() {
    // Pyydä Steamia sulkeutumaan siististi (siivoaa lukitustiedostot)
    let _ = std::process::Command::new("steam").arg("-shutdown").output();
    // Odota max 8 sekuntia että Steam sammuu kokonaan
    for _ in 0..16 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let running = std::process::Command::new("pgrep")
            .args(["-f", "steam.sh"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !running { break; }
    }
    // Pakkosulje jos vielä käynnissä
    let _ = std::process::Command::new("pkill").args(["-9", "-f", "steam.sh"]).output();
    std::thread::sleep(std::time::Duration::from_millis(500));
}

fn start_steam() {
    let _ = std::process::Command::new("steam")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

pub fn spawn_steam_gamemode(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        kill_steam();
        let msg = match steam::add_launch_option("gamemoderun") {
            Ok(m) => { start_steam(); BackendMsg::OperationDone(format!("{m} — Steam käynnistetty uudelleen")) }
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}

pub fn spawn_steam_mangohud(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        kill_steam();
        let msg = match steam::add_launch_option("mangohud") {
            Ok(m) => { start_steam(); BackendMsg::OperationDone(format!("{m} — Steam käynnistetty uudelleen")) }
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

pub fn do_install(pkg: &str) -> Result<String, String> {
    pkexec_install(pkg)
}

fn pkexec_install(pkg: &str) -> Result<String, String> {
    let out = std::process::Command::new("pkexec")
        .args(["zypper", "install", "-y", pkg])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(format!("{pkg} asennettu!"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn spawn_install_gamemode(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        let msg = match pkexec_install("gamemode") {
            Ok(m) => BackendMsg::OperationDone(m),
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}

pub fn spawn_install_mangohud(tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        let msg = match pkexec_install("mangohud") {
            Ok(m) => BackendMsg::OperationDone(m),
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}

pub fn spawn_steam_remove(cmd: &'static str, tx: Sender<BackendMsg>, ctx: Context) {
    std::thread::spawn(move || {
        kill_steam();
        let msg = match steam::remove_launch_option(cmd) {
            Ok(m) => { start_steam(); BackendMsg::OperationDone(format!("{m} — Steam käynnistetty uudelleen")) }
            Err(e) => BackendMsg::Error(e),
        };
        tx.send(msg).ok();
        ctx.request_repaint();
    });
}
