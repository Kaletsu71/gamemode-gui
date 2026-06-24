use crate::backend::{self, BackendMsg};
use egui::{Color32, RichText, Ui};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── Catppuccin Mocha palette ───────────────────────────────────
const ACCENT: Color32 = Color32::from_rgb(0xcb, 0xa6, 0xf7);
const GREEN: Color32 = Color32::from_rgb(0xa6, 0xe3, 0xa1);
const RED: Color32 = Color32::from_rgb(0xf3, 0x8b, 0xa8);
const SUBTEXT: Color32 = Color32::from_rgb(0xba, 0xc2, 0xde);
const TEXT: Color32 = Color32::from_rgb(0xcd, 0xd6, 0xf4);
const BASE: Color32 = Color32::from_rgb(0x1e, 0x1e, 0x2e);
const SURFACE0: Color32 = Color32::from_rgb(0x31, 0x32, 0x44);
const SURFACE1: Color32 = Color32::from_rgb(0x45, 0x47, 0x5a);
const SURFACE2: Color32 = Color32::from_rgb(0x58, 0x5b, 0x70);
const MANTLE: Color32 = Color32::from_rgb(0x18, 0x18, 0x25);
const CRUST: Color32 = Color32::from_rgb(0x11, 0x11, 0x1b);
const OVERLAY0: Color32 = Color32::from_rgb(0xa6, 0xad, 0xc8);

// ── App state ─────────────────────────────────────────────────
pub struct GameModeApp {
    tx: mpsc::Sender<BackendMsg>,
    rx: mpsc::Receiver<BackendMsg>,

    gamemode_status: String,
    mangohud_installed: bool,
    heroic_gm: bool,
    heroic_mh: bool,
    steam_gm: bool,
    steam_mh: bool,
    distro: String,

    status_bar: String,
    busy: bool,
    needs_refresh: bool,
    last_refresh: Instant,
}

impl GameModeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_catppuccin(&cc.egui_ctx);

        let (tx, rx) = mpsc::channel();

        let app = Self {
            tx,
            rx,
            gamemode_status: "—".to_string(),
            mangohud_installed: false,
            heroic_gm: false,
            heroic_mh: false,
            steam_gm: false,
            steam_mh: false,
            distro: "—".to_string(),
            status_bar: "Checking status...".to_string(),
            busy: true,
            needs_refresh: false,
            last_refresh: Instant::now(),
        };

        backend::spawn_status_check(app.tx.clone(), cc.egui_ctx.clone());
        app
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BackendMsg::GameModeStatus(s) => self.gamemode_status = s,
                BackendMsg::MangoHudInstalled(b) => self.mangohud_installed = b,
                BackendMsg::HeroicGmStatus(b) => self.heroic_gm = b,
                BackendMsg::HeroicMhStatus(b) => self.heroic_mh = b,
                BackendMsg::SteamGmStatus(b) => self.steam_gm = b,
                BackendMsg::SteamMhStatus(b) => self.steam_mh = b,
                BackendMsg::Distro(s) => self.distro = s,
                BackendMsg::StatusDone => {
                    self.busy = false;
                    self.status_bar = "Ready".to_string();
                    self.last_refresh = Instant::now();
                }
                BackendMsg::OperationDone(m) => {
                    self.status_bar = m;
                    self.busy = false;
                    self.needs_refresh = true;
                }
                BackendMsg::Error(e) => {
                    self.status_bar = format!("Error: {e}");
                    self.busy = false;
                }
            }
        }
    }
}

// ── egui App ──────────────────────────────────────────────────
impl eframe::App for GameModeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();

        if !self.busy
            && (self.needs_refresh || self.last_refresh.elapsed() >= Duration::from_secs(5))
        {
            backend::spawn_status_check(self.tx.clone(), ctx.clone());
            self.busy = true;
            self.needs_refresh = false;
            self.last_refresh = Instant::now();
        }

        ctx.request_repaint_after(Duration::from_secs(1));

        // ── Status bar (bottom) ────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::none()
                    .fill(CRUST)
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0)),
            )
            .show(ctx, |ui| {
                ui.colored_label(OVERLAY0, &self.status_bar);
            });

        // ── Main scroll area ───────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(MANTLE))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("GameMode Manager")
                                    .size(22.0)
                                    .strong()
                                    .color(ACCENT),
                            );
                        });
                        ui.add_space(14.0);

                        // ── Installation ───────────────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Installation");
                            status_row(ui, "GameMode:", &self.gamemode_status.clone());
                            status_row(
                                ui,
                                "MangoHud:",
                                if self.mangohud_installed { "Installed" } else { "Not installed" },
                            );
                            ui.add_space(6.0);

                            if wide_button(ui, "Check GameMode", !self.busy) {
                                let tx = self.tx.clone();
                                let ctx2 = ctx.clone();
                                self.busy = true;
                                self.status_bar = "Checking GameMode...".to_string();
                                std::thread::spawn(move || {
                                    let s = backend::check_gamemode_status();
                                    tx.send(BackendMsg::GameModeStatus(s)).ok();
                                    tx.send(BackendMsg::StatusDone).ok();
                                    ctx2.request_repaint();
                                });
                            }

                            if wide_button(ui, "Check MangoHud", !self.busy) {
                                let found = std::env::var("PATH")
                                    .unwrap_or_default()
                                    .split(':')
                                    .any(|d| std::path::Path::new(d).join("mangohud").exists());
                                self.mangohud_installed = found;
                                self.status_bar = if found {
                                    "MangoHud: installed".to_string()
                                } else {
                                    "MangoHud: not found in PATH".to_string()
                                };
                            }
                        });

                        ui.add_space(8.0);

                        // ── Steam Integration ──────────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Steam Integration");
                            status_row(
                                ui,
                                "GameMode options:",
                                if self.steam_gm { "ON" } else { "OFF" },
                            );
                            status_row(
                                ui,
                                "MangoHud options:",
                                if self.steam_mh { "ON" } else { "OFF" },
                            );
                            ui.add_space(6.0);

                            if wide_button(ui, "Add GameMode → ALL Steam games", !self.busy) {
                                self.busy = true;
                                self.status_bar = "Adding GameMode to Steam games...".to_string();
                                backend::spawn_steam_gamemode(self.tx.clone(), ctx.clone());
                            }
                            if wide_button(ui, "Add MangoHud → ALL Steam games", !self.busy) {
                                self.busy = true;
                                self.status_bar = "Adding MangoHud to Steam games...".to_string();
                                backend::spawn_steam_mangohud(self.tx.clone(), ctx.clone());
                            }
                        });

                        ui.add_space(8.0);

                        // ── Heroic Games Launcher ──────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Heroic Games Launcher");
                            status_row(
                                ui,
                                "GameMode:",
                                if self.heroic_gm { "ON" } else { "OFF" },
                            );
                            status_row(
                                ui,
                                "MangoHud:",
                                if self.heroic_mh { "ON" } else { "OFF" },
                            );
                            ui.add_space(6.0);

                            let gm = self.heroic_gm;
                            if colored_wide_button(
                                ui,
                                if gm { "Disable GameMode for Heroic" } else { "Enable GameMode for Heroic" },
                                if gm { GREEN } else { RED },
                                !self.busy,
                            ) {
                                let new_val = !gm;
                                self.heroic_gm = new_val;
                                self.busy = true;
                                self.status_bar = "Toggling GameMode for Heroic...".to_string();
                                backend::spawn_heroic_toggle(
                                    "useGameMode".to_string(),
                                    new_val,
                                    self.tx.clone(),
                                    ctx.clone(),
                                );
                            }

                            let mh = self.heroic_mh;
                            if colored_wide_button(
                                ui,
                                if mh { "Disable MangoHud for Heroic" } else { "Enable MangoHud for Heroic" },
                                if mh { GREEN } else { RED },
                                !self.busy,
                            ) {
                                let new_val = !mh;
                                self.heroic_mh = new_val;
                                self.busy = true;
                                self.status_bar = "Toggling MangoHud for Heroic...".to_string();
                                backend::spawn_heroic_toggle(
                                    "enableMangoHud".to_string(),
                                    new_val,
                                    self.tx.clone(),
                                    ctx.clone(),
                                );
                            }

                            if wide_button(ui, "Launch Heroic", true) {
                                let _ = std::process::Command::new("heroic")
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                                self.status_bar = "Heroic launched".to_string();
                            }
                        });

                        ui.add_space(8.0);

                        // ── Live Info ──────────────────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Live Info");
                            status_row(ui, "Distribution:", &self.distro.clone());
                            status_row(ui, "GameMode daemon:", &self.gamemode_status.clone());
                            status_row(
                                ui,
                                "MangoHud:",
                                if self.mangohud_installed { "Installed" } else { "Not installed" },
                            );
                            status_row(
                                ui,
                                "Heroic GameMode:",
                                if self.heroic_gm { "ON" } else { "OFF" },
                            );
                            status_row(
                                ui,
                                "Heroic MangoHud:",
                                if self.heroic_mh { "ON" } else { "OFF" },
                            );
                            status_row(
                                ui,
                                "Steam GameMode:",
                                if self.steam_gm { "ON" } else { "OFF" },
                            );
                            status_row(
                                ui,
                                "Steam MangoHud:",
                                if self.steam_mh { "ON" } else { "OFF" },
                            );
                        });

                        ui.add_space(16.0);
                    });
            });
    }
}

// ── UI helpers ─────────────────────────────────────────────────

fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BASE)
        .stroke(egui::Stroke::new(1.0, SURFACE0))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .outer_margin(egui::Margin::symmetric(12.0, 0.0))
}

fn card_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).size(14.0).strong().color(ACCENT));
    ui.separator();
    ui.add_space(4.0);
}

fn status_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(SUBTEXT, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let color = match value.to_ascii_uppercase().as_str() {
                "ON" | "INSTALLED" | "YES" => GREEN,
                "OFF" | "NOT INSTALLED" | "NOT FOUND" | "NO" => RED,
                v if v.starts_with("NOT") => RED,
                _ => TEXT,
            };
            ui.colored_label(color, RichText::new(value).strong());
        });
    });
    ui.add_space(2.0);
}

fn wide_button(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(label).min_size(egui::vec2(ui.available_width(), 30.0)),
    )
    .clicked()
}

fn colored_wide_button(ui: &mut Ui, label: &str, color: Color32, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).color(color))
            .min_size(egui::vec2(ui.available_width(), 30.0)),
    )
    .clicked()
}

// ── Catppuccin theme ───────────────────────────────────────────

pub fn apply_catppuccin(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();

    v.panel_fill = MANTLE;
    v.window_fill = BASE;
    v.extreme_bg_color = CRUST;
    v.faint_bg_color = BASE;

    v.widgets.noninteractive.bg_fill = SURFACE0;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_fill = SURFACE1;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    v.widgets.hovered.bg_fill = SURFACE2;
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, TEXT);
    v.widgets.active.bg_fill = ACCENT;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.5, BASE);

    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xcb, 0xa6, 0xf7, 0x60);
    v.hyperlink_color = ACCENT;
    v.window_stroke = egui::Stroke::new(1.0, SURFACE0);
    v.override_text_color = Some(TEXT);

    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    ctx.set_style(style);
}
