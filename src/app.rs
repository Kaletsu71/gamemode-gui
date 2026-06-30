use crate::backend::{self, BackendMsg};
use egui::{Color32, RichText, Ui};
use std::sync::mpsc;
use std::time::{Duration, Instant};

// ── Catppuccin Mocha -väripaletti ─────────────────────────────
const ACCENT: Color32  = Color32::from_rgb(0xcb, 0xa6, 0xf7); // violetti
const GREEN: Color32   = Color32::from_rgb(0xa6, 0xe3, 0xa1);
const RED: Color32     = Color32::from_rgb(0xf3, 0x8b, 0xa8);
const YELLOW: Color32  = Color32::from_rgb(0xf9, 0xe2, 0xaf);
const SUBTEXT: Color32 = Color32::from_rgb(0xba, 0xc2, 0xde);
const TEXT: Color32    = Color32::from_rgb(0xcd, 0xd6, 0xf4);
const BASE: Color32    = Color32::from_rgb(0x1e, 0x1e, 0x2e);
const SURFACE0: Color32= Color32::from_rgb(0x31, 0x32, 0x44);
const SURFACE1: Color32= Color32::from_rgb(0x45, 0x47, 0x5a);
const SURFACE2: Color32= Color32::from_rgb(0x58, 0x5b, 0x70);
const MANTLE: Color32  = Color32::from_rgb(0x18, 0x18, 0x25);
const CRUST: Color32   = Color32::from_rgb(0x11, 0x11, 0x1b);
const OVERLAY0: Color32= Color32::from_rgb(0xa6, 0xad, 0xc8);

// ── Sovellustila ───────────────────────────────────────────────
pub struct GameModeApp {
    // Kanava taustasäikeistä UI:hin viestien lähettämiseen
    tx: mpsc::Sender<BackendMsg>,
    rx: mpsc::Receiver<BackendMsg>,

    // Tilamuuttujat — päivittyvät taustasäikeistä
    gamemode_status: String,   // "ON", "OFF" tai "Not installed"
    mangohud_installed: bool,
    heroic_installed: bool,
    heroic_gm: bool,
    heroic_mh: bool,
    steam_gm: bool,
    steam_mh: bool,
    ai_hook: bool,
    distro: String,

    // UI-tila
    status_bar: String,
    busy: bool,          // true kun taustasäie on käynnissä, napit disabled
    needs_refresh: bool, // true kun tila pitää päivittää heti
    last_refresh: Instant,

    // Ilmoitusviesti: (teksti, väri, aika jolloin näytetään)
    // Näkyy 4 sekuntia operaation jälkeen
    notification: Option<(String, Color32, Instant)>,
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
            heroic_installed: false,
            heroic_gm: false,
            heroic_mh: false,
            steam_gm: false,
            steam_mh: false,
            ai_hook: false,
            distro: "—".to_string(),
            status_bar: "Tarkistetaan tilaa...".to_string(),
            busy: true,
            needs_refresh: false,
            last_refresh: Instant::now(),
            notification: None,
        };

        // Käynnistä ensimmäinen statustarkistus heti
        backend::spawn_status_check(app.tx.clone(), cc.egui_ctx.clone());
        app
    }

    /// Lukee kaikki odottavat viestit taustasäikeiltä ja päivittää tilan
    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                BackendMsg::GameModeStatus(s) => self.gamemode_status = s,
                BackendMsg::MangoHudInstalled(b) => self.mangohud_installed = b,
                BackendMsg::HeroicInstalled(b) => self.heroic_installed = b,
                BackendMsg::HeroicGmStatus(b)  => self.heroic_gm = b,
                BackendMsg::HeroicMhStatus(b)  => self.heroic_mh = b,
                BackendMsg::SteamGmStatus(b)   => self.steam_gm = b,
                BackendMsg::SteamMhStatus(b)   => self.steam_mh = b,
                BackendMsg::AiHookStatus(b)    => self.ai_hook = b,
                BackendMsg::Distro(s)          => self.distro = s,
                BackendMsg::StatusDone => {
                    self.busy = false;
                    self.status_bar = "Valmis".to_string();
                    self.last_refresh = Instant::now();
                }
                BackendMsg::OperationDone(m) => {
                    // Näytä onnistumisviesti 4 sekuntia
                    self.notification = Some((m.clone(), GREEN, Instant::now()));
                    self.status_bar = m;
                    self.busy = false;
                    self.needs_refresh = true;
                }
                BackendMsg::Error(e) => {
                    // Näytä virheviesti punaisena 4 sekuntia
                    self.notification = Some((format!("Virhe: {e}"), RED, Instant::now()));
                    self.status_bar = format!("Virhe: {e}");
                    self.busy = false;
                }
            }
        }
    }

    /// Piirtää ilmoitusbannerin jos sellainen on aktiivinen
    fn show_notification(&mut self, ctx: &egui::Context) {
        let Some((msg, color, started)) = &self.notification else { return };
        if started.elapsed() > Duration::from_secs(4) {
            self.notification = None;
            return;
        }
        // Näytä ilmoitus ikkunan yläreunassa
        egui::TopBottomPanel::top("notification")
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgba_unmultiplied(
                        color.r(), color.g(), color.b(), 40,
                    ))
                    .stroke(egui::Stroke::new(1.5, *color))
                    .inner_margin(egui::Margin::symmetric(16.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(*color, RichText::new("●").size(14.0));
                    ui.colored_label(*color, RichText::new(msg).size(13.0).strong());
                });
            });
    }
}

// ── egui App -implementaatio ───────────────────────────────────
impl eframe::App for GameModeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();

        // Automaattinen päivitys 5 sekunnin välein
        if !self.busy
            && (self.needs_refresh || self.last_refresh.elapsed() >= Duration::from_secs(5))
        {
            backend::spawn_status_check(self.tx.clone(), ctx.clone());
            self.busy = true;
            self.needs_refresh = false;
            self.last_refresh = Instant::now();
        }

        // Pakota piirto 1 sekunnin välein (ilmoitusbannerin katoamista varten)
        ctx.request_repaint_after(Duration::from_secs(1));

        // Ilmoitusbannerи (piirretään ennen muita paneeleja)
        self.show_notification(ctx);

        // ── Alhaalla oleva statusrivi ──────────────────────────
        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::none()
                    .fill(CRUST)
                    .inner_margin(egui::Margin::symmetric(12.0, 5.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Vilkkuva piste kun käynnissä
                    if self.busy {
                        ui.colored_label(YELLOW, "⟳");
                    }
                    ui.colored_label(OVERLAY0, &self.status_bar);
                });
            });

        // ── Pääsisältöalue ─────────────────────────────────────
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

                        // ── Asennus-kortti ─────────────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Asennus");

                            // Näytä asennustila (ei daemonin tila)
                            status_row(ui, "GameMode:",
                                if self.gamemode_status == "Not installed" { "Ei asennettu" } else { "Asennettu" }
                            );
                            status_row(
                                ui,
                                "MangoHud:",
                                if self.mangohud_installed { "Asennettu" } else { "Ei asennettu" },
                            );
                            ui.add_space(6.0);

                            // GameMode: jos ei ole asennettu → asenna, muuten → tarkista
                            if self.gamemode_status == "Not installed" {
                                if colored_wide_button(ui, "⬇ Asenna GameMode", ACCENT, !self.busy) {
                                    self.busy = true;
                                    self.status_bar = "Asennetaan GameMode...".to_string();
                                    // pkexec avaa salasanadialogissa
                                    backend::spawn_install_gamemode(self.tx.clone(), ctx.clone());
                                }
                            } else if wide_button(ui, "↺ Tarkista GameMode", !self.busy) {
                                let tx = self.tx.clone();
                                let ctx2 = ctx.clone();
                                self.busy = true;
                                self.status_bar = "Tarkistetaan GameMode...".to_string();
                                std::thread::spawn(move || {
                                    // Tarkista onko gamemoded-binääri asennettu PATH:iin
                                    let installed = std::env::var("PATH").unwrap_or_default()
                                        .split(':')
                                        .any(|d| std::path::Path::new(d).join("gamemoded").exists());
                                    tx.send(BackendMsg::GameModeStatus(
                                        if installed { "OFF".to_string() } else { "Not installed".to_string() }
                                    )).ok();
                                    tx.send(BackendMsg::OperationDone(
                                        if installed { "GameMode: asennettu ✓".to_string() }
                                        else { "GameMode: ei asennettu".to_string() }
                                    )).ok();
                                    ctx2.request_repaint();
                                });
                            }

                            ui.add_space(4.0);

                            // MangoHud: jos ei ole asennettu → asenna, muuten → tarkista
                            if !self.mangohud_installed {
                                if colored_wide_button(ui, "⬇ Asenna MangoHud", ACCENT, !self.busy) {
                                    self.busy = true;
                                    self.status_bar = "Asennetaan MangoHud...".to_string();
                                    backend::spawn_install_mangohud(self.tx.clone(), ctx.clone());
                                }
                            } else if wide_button(ui, "↺ Tarkista MangoHud", !self.busy) {
                                let tx = self.tx.clone();
                                let ctx2 = ctx.clone();
                                self.busy = true;
                                self.status_bar = "Tarkistetaan MangoHud...".to_string();
                                std::thread::spawn(move || {
                                    let found = std::env::var("PATH")
                                        .unwrap_or_default()
                                        .split(':')
                                        .any(|d| std::path::Path::new(d).join("mangohud").exists());
                                    tx.send(BackendMsg::MangoHudInstalled(found)).ok();
                                    tx.send(BackendMsg::OperationDone(
                                        if found { "MangoHud: asennettu ✓".to_string() }
                                        else      { "MangoHud: ei löydy — klikkaa Asenna".to_string() }
                                    )).ok();
                                    ctx2.request_repaint();
                                });
                            }
                        });

                        ui.add_space(8.0);

                        // ── Steam-integraatio ──────────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Steam-integraatio");
                            status_row(
                                ui,
                                "GameMode käynnistysoptiot:",
                                if self.steam_gm { "ON" } else { "OFF" },
                            );
                            status_row(
                                ui,
                                "MangoHud käynnistysoptiot:",
                                if self.steam_mh { "ON" } else { "OFF" },
                            );
                            ui.add_space(6.0);

                            // Lisää/poista gamemoderun kaikkiin Steam-peleihin
                            if colored_wide_button(ui, "➕ Lisää GameMode → kaikki Steam-pelit", GREEN, !self.busy) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Steam ja lisätään GameMode...".to_string();
                                backend::spawn_steam_gamemode(self.tx.clone(), ctx.clone());
                            }
                            if colored_wide_button(ui, "➖ Poista GameMode ← kaikki Steam-pelit", RED, !self.busy) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Steam ja poistetaan GameMode...".to_string();
                                backend::spawn_steam_remove("gamemoderun", self.tx.clone(), ctx.clone());
                            }
                            ui.add_space(4.0);
                            // Lisää/poista mangohud kaikkiin Steam-peleihin
                            if colored_wide_button(ui, "➕ Lisää MangoHud → kaikki Steam-pelit", GREEN, !self.busy) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Steam ja lisätään MangoHud...".to_string();
                                backend::spawn_steam_mangohud(self.tx.clone(), ctx.clone());
                            }
                            if colored_wide_button(ui, "➖ Poista MangoHud ← kaikki Steam-pelit", RED, !self.busy) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Steam ja poistetaan MangoHud...".to_string();
                                backend::spawn_steam_remove("mangohud", self.tx.clone(), ctx.clone());
                            }
                        });

                        ui.add_space(8.0);

                        // ── Heroic Games Launcher ──────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Heroic Games Launcher");
                            status_row(ui, "Heroic:",
                                if self.heroic_installed { "Asennettu" } else { "Ei asennettu" }
                            );
                            status_row(ui, "GameMode:", if self.heroic_gm { "ON" } else { "OFF" });
                            status_row(ui, "MangoHud:", if self.heroic_mh { "ON" } else { "OFF" });
                            ui.add_space(6.0);

                            let enabled = !self.busy && self.heroic_installed;

                            // GameMode: erilliset Lisää/Poista-napit (kuten Steamissa)
                            if colored_wide_button(ui, "➕ Lisää GameMode → Heroic", GREEN, enabled) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Heroic ja lisätään GameMode...".to_string();
                                backend::spawn_heroic_toggle(
                                    "useGameMode".to_string(), true,
                                    self.tx.clone(), ctx.clone(),
                                );
                            }
                            if colored_wide_button(ui, "➖ Poista GameMode ← Heroic", RED, enabled) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Heroic ja poistetaan GameMode...".to_string();
                                backend::spawn_heroic_toggle(
                                    "useGameMode".to_string(), false,
                                    self.tx.clone(), ctx.clone(),
                                );
                            }
                            ui.add_space(4.0);
                            // MangoHud: erilliset Lisää/Poista-napit
                            if colored_wide_button(ui, "➕ Lisää MangoHud → Heroic", GREEN, enabled) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Heroic ja lisätään MangoHud...".to_string();
                                backend::spawn_heroic_toggle(
                                    "enableMangoHud".to_string(), true,
                                    self.tx.clone(), ctx.clone(),
                                );
                            }
                            if colored_wide_button(ui, "➖ Poista MangoHud ← Heroic", RED, enabled) {
                                self.busy = true;
                                self.status_bar = "Suljetaan Heroic ja poistetaan MangoHud...".to_string();
                                backend::spawn_heroic_toggle(
                                    "enableMangoHud".to_string(), false,
                                    self.tx.clone(), ctx.clone(),
                                );
                            }

                            ui.add_space(4.0);
                            if wide_button(ui, "🚀 Käynnistä Heroic", !self.busy) {
                                if self.heroic_installed {
                                    let _ = std::process::Command::new("heroic")
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .spawn();
                                    self.notification = Some((
                                        "Heroic käynnistetty".to_string(), ACCENT, Instant::now()
                                    ));
                                } else {
                                    self.notification = Some((
                                        "Heroic ei ole asennettu".to_string(), RED, Instant::now()
                                    ));
                                }
                            }
                        });

                        ui.add_space(8.0);

                        // ── AI models (llama.cpp) while gaming ─────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "AI-mallit pelatessa");
                            status_row(ui, "VRAM-hook:", if self.ai_hook { "ON" } else { "OFF" });
                            ui.add_space(4.0);
                            ui.colored_label(
                                SUBTEXT,
                                RichText::new(
                                    "Sammuttaa paikalliset llama.cpp-mallit pelin ajaksi ja \
                                     palauttaa ne jälkeen — vapauttaa VRAM:n pelille.",
                                )
                                .size(11.0),
                            );
                            ui.add_space(6.0);

                            // Install/remove the gamemode.ini [custom] hook
                            if colored_wide_button(ui, "➕ Lisää AI-VRAM-hook → GameMode", GREEN, !self.busy) {
                                self.busy = true;
                                self.status_bar = "Lisätään AI-VRAM-hook...".to_string();
                                backend::spawn_install_ai_hook(self.tx.clone(), ctx.clone());
                            }
                            if colored_wide_button(ui, "➖ Poista AI-VRAM-hook ← GameMode", RED, !self.busy) {
                                self.busy = true;
                                self.status_bar = "Poistetaan AI-VRAM-hook...".to_string();
                                backend::spawn_remove_ai_hook(self.tx.clone(), ctx.clone());
                            }
                        });

                        ui.add_space(8.0);

                        // ── Live-tilannäyttö ───────────────────
                        card_frame().show(ui, |ui| {
                            card_title(ui, "Tila");
                            status_row(ui, "Jakelu:", &self.distro.clone());
                            status_row(ui, "GameMode daemon:", &self.gamemode_status.clone());
                            status_row(
                                ui, "MangoHud:",
                                if self.mangohud_installed { "Asennettu" } else { "Ei asennettu" },
                            );
                            status_row(ui, "Heroic GameMode:", if self.heroic_gm { "ON" } else { "OFF" });
                            status_row(ui, "Heroic MangoHud:", if self.heroic_mh { "ON" } else { "OFF" });
                            status_row(ui, "Steam GameMode:",  if self.steam_gm  { "ON" } else { "OFF" });
                            status_row(ui, "Steam MangoHud:",  if self.steam_mh  { "ON" } else { "OFF" });
                            status_row(ui, "AI-VRAM-hook:",    if self.ai_hook   { "ON" } else { "OFF" });
                        });

                        ui.add_space(16.0);
                    });
            });
    }
}

// ── UI-apufunktiot ─────────────────────────────────────────────

/// Kortin kehys (pyöristetyt kulmat, BASE-tausta)
fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BASE)
        .stroke(egui::Stroke::new(1.0, SURFACE0))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .outer_margin(egui::Margin::symmetric(12.0, 0.0))
}

/// Kortin otsikko violetilla värillä ja erotusviiivalla
fn card_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).size(14.0).strong().color(ACCENT));
    ui.separator();
    ui.add_space(4.0);
}

/// Statusrivi: "Nimi:   ARVO" — väri arvon mukaan (vihreä/punainen/valkoinen)
fn status_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(SUBTEXT, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let color = match value.to_ascii_uppercase().as_str() {
                "ON" | "ASENNETTU" | "INSTALLED" => GREEN,
                "OFF" | "EI ASENNETTU" | "NOT INSTALLED" => RED,
                v if v.starts_with("NOT") || v.starts_with("EI") => RED,
                _ => TEXT,
            };
            ui.colored_label(color, RichText::new(value).strong());
        });
    });
    ui.add_space(2.0);
}

/// Koko leveyden nappi (harmaa, disabled-tuki)
fn wide_button(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(label).min_size(egui::vec2(ui.available_width(), 32.0)),
    )
    .clicked()
}

/// Koko leveyden nappi värilisellä tekstillä
fn colored_wide_button(ui: &mut Ui, label: &str, color: Color32, enabled: bool) -> bool {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).color(color))
            .min_size(egui::vec2(ui.available_width(), 32.0)),
    )
    .clicked()
}

// ── Catppuccin Mocha -teema ────────────────────────────────────
pub fn apply_catppuccin(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();

    v.panel_fill         = MANTLE;
    v.window_fill        = BASE;
    v.extreme_bg_color   = CRUST;
    v.faint_bg_color     = BASE;

    v.widgets.noninteractive.bg_fill  = SURFACE0;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    v.widgets.inactive.bg_fill        = SURFACE1;
    v.widgets.inactive.fg_stroke      = egui::Stroke::new(1.0, TEXT);
    v.widgets.hovered.bg_fill         = SURFACE2;
    v.widgets.hovered.fg_stroke       = egui::Stroke::new(1.5, TEXT);
    v.widgets.active.bg_fill          = ACCENT;
    v.widgets.active.fg_stroke        = egui::Stroke::new(1.5, BASE);

    v.selection.bg_fill  = Color32::from_rgba_unmultiplied(0xcb, 0xa6, 0xf7, 0x60);
    v.hyperlink_color    = ACCENT;
    v.window_stroke      = egui::Stroke::new(1.0, SURFACE0);
    v.override_text_color = Some(TEXT);

    style.visuals = v;
    style.spacing.item_spacing   = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);

    ctx.set_style(style);
}
