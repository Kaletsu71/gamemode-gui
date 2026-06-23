"""GameMode Manager — Qt6 / PySide6 GUI."""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

from PySide6.QtCore import QObject, QThread, QTimer, Qt, Signal, Slot
from PySide6.QtGui import QColor, QFont, QIcon, QPalette
from PySide6.QtWidgets import (
    QApplication,
    QCheckBox,
    QFrame,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QMainWindow,
    QPushButton,
    QScrollArea,
    QSizePolicy,
    QStatusBar,
    QVBoxLayout,
    QWidget,
)

# ── Constants ──────────────────────────────────────────────────
APP_NAME = "GameMode Manager"
ICON_PATH = Path(__file__).parent / "assets" / "gamemode-manager.svg"
GAMEMODE_SCRIPT = Path.home() / "Documents" / "gamemode"
STEAM_VDF = Path.home() / ".local" / "share" / "Steam" / "userdata" / "1092453251" / "config" / "localconfig.vdf"
HEROIC_CFG = Path.home() / ".config" / "heroic" / "config.json"
REFRESH_MS = 5000

CONFIG_DIR = Path.home() / ".config" / "gamemode-manager"
CONFIG_FILE = CONFIG_DIR / "config.json"
LOG_DIR = Path.home() / ".local" / "share" / "gamemode-manager"
LOG_FILE = LOG_DIR / "app.log"


def _ensure_dirs() -> None:
    CONFIG_DIR.mkdir(parents=True, exist_ok=True)
    LOG_DIR.mkdir(parents=True, exist_ok=True)


def log(msg: str) -> None:
    _ensure_dirs()
    ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    with open(LOG_FILE, "a") as f:
        f.write(f"[{ts}] {msg}\n")


def load_config() -> dict[str, Any]:
    _ensure_dirs()
    if CONFIG_FILE.exists():
        try:
            return json.loads(CONFIG_FILE.read_text())
        except Exception:
            pass
    return {"steam_launch_applied": False, "last_check": {}}


def save_config(cfg: dict[str, Any]) -> None:
    _ensure_dirs()
    CONFIG_FILE.write_text(json.dumps(cfg, indent=2))


# ── Workers ────────────────────────────────────────────────────
class _BaseWorker(QObject):
    finished = Signal()
    failed = Signal(str)

    def _run(self, fn):
        try:
            fn()
        except Exception as e:
            self.failed.emit(str(e))
            log(f"Worker error: {e}")
        finally:
            self.finished.emit()


class StatusWorker(_BaseWorker):
    gamemode_status = Signal(str)
    mangohud_heroic_status = Signal(str)
    mangohud_steam_status = Signal(str)
    steam_gm_status = Signal(str)
    distro_name = Signal(str)

    @Slot()
    def run(self):
        def work():
            gm = "OFF"
            try:
                res = subprocess.run(["gamemoded", "-s"], capture_output=True, text=True, timeout=5)
                out = res.stdout.lower()
                if "active" in out or "on" in out:
                    gm = "ON"
                else:
                    gm = res.stdout.strip() or "OFF"
            except Exception:
                gm = "Not installed"
            self.gamemode_status.emit(gm)

            heroic_mh = "OFF"
            try:
                if HEROIC_CFG.exists():
                    data = json.loads(HEROIC_CFG.read_text())
                    for sec in ("defaultSettings", "settings"):
                        if sec in data and "enableMangoHud" in data[sec]:
                            heroic_mh = "ON" if data[sec]["enableMangoHud"] else "OFF"
                            break
                    else:
                        heroic_mh = "Not set"
                else:
                    heroic_mh = "Config not found"
            except Exception:
                heroic_mh = "Error"
            self.mangohud_heroic_status.emit(heroic_mh)

            steam_gm = "OFF"
            steam_mh = "OFF"
            try:
                if STEAM_VDF.exists():
                    text = STEAM_VDF.read_text(errors="replace")
                    steam_gm = "ON" if "gamemoderun" in text else "OFF"
                    steam_mh = "ON" if "mangohud" in text else "OFF"
                else:
                    steam_gm = "Not found"
                    steam_mh = "Not found"
            except Exception:
                steam_gm = "Error"
                steam_mh = "Error"
            self.steam_gm_status.emit(steam_gm)
            self.mangohud_steam_status.emit(steam_mh)

            distro = "Unknown"
            try:
                for line in Path("/etc/os-release").read_text().splitlines():
                    if line.startswith("PRETTY_NAME="):
                        distro = line.split("=", 1)[1].strip().strip('"')
                        break
            except Exception:
                pass
            self.distro_name.emit(distro)

            log(f"Status: GM={gm}, MH_H={heroic_mh}, MH_S={steam_mh}, Distro={distro}")

        self._run(work)


class ToggleWorker(_BaseWorker):
    toggled = Signal(bool, str)

    def __init__(self, key: str, enable: bool):
        super().__init__()
        self.key = key
        self.enable = enable

    @Slot()
    def run(self):
        def work():
            if not HEROIC_CFG.exists():
                self.toggled.emit(False, "Heroic config not found")
                return
            data = json.loads(HEROIC_CFG.read_text())
            updated = False
            for sec in ("defaultSettings", "settings"):
                if sec in data and self.key in data[sec] and data[sec][self.key] != self.enable:
                    data[sec][self.key] = self.enable
                    updated = True
                    break
            if not updated:
                for sec in ("defaultSettings", "settings"):
                    if sec in data:
                        data[sec][self.key] = self.enable
                        updated = True
                        break
            if not updated:
                data[self.key] = self.enable
            HEROIC_CFG.write_text(json.dumps(data, indent=2))

            # Also update per-game overrides in GamesConfig/*.json
            # Heroic uses different key names: enableMangoHud in defaultSettings,
            # but showMangohud in per-game overrides.
            games_dir = Path.home() / ".config" / "heroic" / "GamesConfig"
            game_updated = 0
            if games_dir.is_dir():
                for gf in games_dir.glob("*.json"):
                    if gf.name.endswith(".bak"):
                        continue
                    try:
                        gdata = json.loads(gf.read_text())
                        for app_id, cfg in gdata.items():
                            if not isinstance(cfg, dict):
                                continue
                            changed = False
                            # Map MangoHud key between configs
                            if self.key == "enableMangoHud":
                                if "showMangohud" in cfg and cfg["showMangohud"] != self.enable:
                                    cfg["showMangohud"] = self.enable
                                    changed = True
                                if "enableMangoHud" in cfg and cfg["enableMangoHud"] != self.enable:
                                    cfg["enableMangoHud"] = self.enable
                                    changed = True
                            elif self.key == "showMangohud":
                                if "enableMangoHud" in cfg and cfg["enableMangoHud"] != self.enable:
                                    cfg["enableMangoHud"] = self.enable
                                    changed = True
                                if "showMangohud" in cfg and cfg["showMangohud"] != self.enable:
                                    cfg["showMangohud"] = self.enable
                                    changed = True
                            else:
                                if self.key in cfg and cfg[self.key] != self.enable:
                                    cfg[self.key] = self.enable
                                    changed = True
                            if changed:
                                game_updated += 1
                        if game_updated:
                            gf.write_text(json.dumps(gdata, indent=2))
                    except Exception as e:
                        log(f"ToggleWorker game update error {gf.name}: {e}")
                        pass

            self.toggled.emit(True, f"{self.key} {'enabled' if self.enable else 'disabled'} ({game_updated} games)")
            log(f"Heroic {self.key} -> {self.enable}, updated {game_updated} game configs")

        self._run(work)


class SteamWorker(_BaseWorker):
    applied = Signal(bool, str)

    def __init__(self, enable: bool):
        super().__init__()
        self.enable = enable

    @Slot()
    def run(self):
        def work():
            if not STEAM_VDF.exists():
                self.applied.emit(False, "Steam VDF not found")
                return
            text = STEAM_VDF.read_text(errors="replace")
            import re
            app_pat = re.compile(r'^\s*"(\d+)"\s*\{', re.MULTILINE)
            changed_apps: set[str] = set()

            for m in app_pat.finditer(text):
                app_id = m.group(1)
                start = m.end() - 1  # position of {
                depth = 0
                end = start
                for i in range(start, len(text)):
                    if text[i] == '{':
                        depth += 1
                    elif text[i] == '}':
                        depth -= 1
                        if depth == 0:
                            end = i
                            break
                block = text[start:end+1]
                # Match existing LaunchOptions value line
                if re.search(r'"LaunchOptions"\s+"[^"]*"', block):
                    continue
                changed_apps.add(app_id)
                # Insert after the opening { using the same indent as existing block members
                insert_pos = start + 1
                launch_line = '\t\t\t\t\t"LaunchOptions"\t\t"gamemoderun %command%"\n'
                text = text[:insert_pos] + launch_line + text[insert_pos:]

            if not changed_apps:
                state = "already applied" if self.enable else "already clean"
                self.applied.emit(True, f"Steam GameMode {state}")
                log(f"Steam GameMode options no-op: {state}")
                return

            STEAM_VDF.write_text(text)
            self.applied.emit(True, f"GameMode added to {len(changed_apps)} Steam games")
            log(f"Steam GameMode added to apps: {sorted(changed_apps)}")

        self._run(work)


class SteamMangoHudWorker(_BaseWorker):
    applied = Signal(bool, str)

    def __init__(self, enable: bool):
        super().__init__()
        self.enable = enable

    @Slot()
    def run(self):
        def work():
            if not STEAM_VDF.exists():
                self.applied.emit(False, "Steam VDF not found")
                return
            text = STEAM_VDF.read_text(errors="replace")
            import re
            app_pat = re.compile(r'^\s*"(\d+)"\s*\{', re.MULTILINE)
            changed_apps: set[str] = set()

            for m in app_pat.finditer(text):
                app_id = m.group(1)
                start = m.end() - 1
                depth = 0
                end = start
                for i in range(start, len(text)):
                    if text[i] == '{':
                        depth += 1
                    elif text[i] == '}':
                        depth -= 1
                        if depth == 0:
                            end = i
                            break
                block = text[start:end+1]
                if re.search(r'"LaunchOptions"\s+"[^"]*"', block):
                    continue
                changed_apps.add(app_id)
                insert_pos = start + 1
                launch_line = '\t\t\t\t\t"LaunchOptions"\t\t"mangohud %command%"\n'
                text = text[:insert_pos] + launch_line + text[insert_pos:]

            if not changed_apps:
                state = "already applied" if self.enable else "already clean"
                self.applied.emit(True, f"Steam MangoHud {state}")
                log(f"Steam MangoHud options no-op: {state}")
                return

            STEAM_VDF.write_text(text)
            self.applied.emit(True, f"MangoHud added to {len(changed_apps)} Steam games")
            log(f"Steam MangoHud added to apps: {sorted(changed_apps)}")

        self._run(work)


class InstallWorker(_BaseWorker):
    checked = Signal(str, str)  # name, status

    def __init__(self, name: str, cmd: list[str]):
        super().__init__()
        self.name = name
        self.cmd = cmd

    @Slot()
    def run(self):
        def work():
            self.checked.emit(self.name, "checking")
            try:
                res = subprocess.run(self.cmd, capture_output=True, text=True, timeout=10)
                self.checked.emit(self.name, "installed" if res.returncode == 0 else "not installed")
            except Exception:
                self.checked.emit(self.name, "not installed")

        self._run(work)


# ── UI helpers ─────────────────────────────────────────────────
class Card(QGroupBox):
    """Dark card with border + padding."""

    def __init__(self, title: str, parent: QWidget | None = None):
        super().__init__(title, parent)
        self.setFlat(True)
        self.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Fixed)
        self.setStyleSheet(
            """
            QGroupBox {
                border: 1px solid #313244;
                border-radius: 10px;
                margin-top: 12px;
                padding-top: 20px;
                background: #1e1e2e;
            }
            QGroupBox::title {
                subcontrol-origin: margin;
                left: 14px;
                padding: 0 6px;
                color: #cba6f7;
                font-weight: bold;
                font-size: 14px;
            }
            """
        )
        self._layout = QVBoxLayout(self)
        self._layout.setSpacing(8)
        self._layout.setContentsMargins(16, 6, 16, 16)

    def add_row(self, label: str, widget: QWidget) -> None:
        row = QWidget()
        row_layout = QHBoxLayout(row)
        row_layout.setContentsMargins(0, 0, 0, 0)
        row_layout.setSpacing(12)
        lbl = QLabel(label)
        lbl.setMinimumWidth(200)
        lbl.setStyleSheet("color: #bac2de;")
        row_layout.addWidget(lbl)
        row_layout.addWidget(widget, 1)
        self._layout.addWidget(row)


class StatusLabel(QLabel):
    """Color-coded status label."""

    _STYLE = {
        "checking": ("⏳ Checking...", "#f9e2af"),
        "installed": ("✅ Installed", "#a6e3a1"),
        "not installed": ("❌ Not installed", "#f38ba8"),
        "applied": ("✅ Applied", "#a6e3a1"),
        "not applied": ("❌ Not applied", "#f38ba8"),
        "on": ("ON", "#a6e3a1"),
        "off": ("OFF", "#f38ba8"),
    }

    def set_status(self, text: str) -> None:
        text = text.strip()
        if text.lower() in self._STYLE:
            label, color = self._STYLE[text.lower()]
        else:
            label, color = text, "#cdd6f4"
        self.setText(label)
        self.setStyleSheet(f"color: {color}; font-weight: bold;")


# ── Main Window ────────────────────────────────────────────────
def _apply_dark_palette(app: QApplication) -> None:
    palette = QPalette()
    palette.setColor(QPalette.Window, QColor("#1e1e2e"))
    palette.setColor(QPalette.WindowText, QColor("#cdd6f4"))
    palette.setColor(QPalette.Base, QColor("#181825"))
    palette.setColor(QPalette.AlternateBase, QColor("#313244"))
    palette.setColor(QPalette.Text, QColor("#cdd6f4"))
    palette.setColor(QPalette.Button, QColor("#45475a"))
    palette.setColor(QPalette.ButtonText, QColor("#cdd6f4"))
    palette.setColor(QPalette.Highlight, QColor("#585b70"))
    palette.setColor(QPalette.HighlightedText, QColor("#cdd6f4"))
    app.setPalette(palette)
    app.setStyle("Fusion")
    font = QFont("Segoe UI", 10)
    if sys.platform.startswith("linux"):
        font.setFamily("Noto Sans")
    app.setFont(font)


class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle(APP_NAME)
        self.setMinimumSize(500, 600)
        if ICON_PATH.exists():
            self.setWindowIcon(QIcon(str(ICON_PATH)))

        self.cfg = load_config()
        self._active: list[tuple[QThread, _BaseWorker]] = []

        # Central scroll area
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setFrameShape(QFrame.NoFrame)
        central = QWidget()
        scroll.setWidget(central)
        self.setCentralWidget(scroll)

        root = QVBoxLayout(central)
        root.setSpacing(14)
        root.setContentsMargins(20, 20, 20, 20)

        # Title
        title = QLabel(APP_NAME)
        title.setStyleSheet("font-size: 22px; font-weight: bold; color: #cba6f7;")
        title.setAlignment(Qt.AlignmentFlag.AlignHCenter)
        root.addWidget(title)

        # ── Installation ──
        inst = Card("Installation")
        self.gm_status = StatusLabel("Not checked")
        self.mh_status = StatusLabel("Not checked")
        inst.add_row("GameMode:", self.gm_status)
        inst.add_row("MangoHud:", self.mh_status)

        btn_gm = QPushButton("Check / Install GameMode")
        btn_gm.clicked.connect(self._check_gamemode)
        inst._layout.addWidget(btn_gm)

        btn_mh = QPushButton("Check / Install MangoHud")
        btn_mh.clicked.connect(self._check_mangohud)
        inst._layout.addWidget(btn_mh)

        root.addWidget(inst)

        # ── Steam ──
        steam = Card("Steam Integration")
        self.steam_gm_status = StatusLabel("Not applied")
        self.steam_mh_status = StatusLabel("Not applied")
        steam.add_row("GameMode launch options:", self.steam_gm_status)
        steam.add_row("MangoHud launch options:", self.steam_mh_status)

        btn_steam_add = QPushButton("Add GameMode launch options to ALL Steam games")
        btn_steam_add.clicked.connect(lambda: self._steam_options(True))
        steam._layout.addWidget(btn_steam_add)

        btn_steam_rm = QPushButton("Remove GameMode from Steam games")
        btn_steam_rm.clicked.connect(lambda: self._steam_options(False))
        steam._layout.addWidget(btn_steam_rm)

        btn_steam_mh_add = QPushButton("Add MangoHud launch options to ALL Steam games")
        btn_steam_mh_add.clicked.connect(lambda: self._steam_mh_options(True))
        steam._layout.addWidget(btn_steam_mh_add)

        btn_steam_mh_rm = QPushButton("Remove MangoHud from Steam games")
        btn_steam_mh_rm.clicked.connect(lambda: self._steam_mh_options(False))
        steam._layout.addWidget(btn_steam_mh_rm)

        root.addWidget(steam)

        # ── Heroic ──
        heroic = Card("Heroic Games Launcher")
        self.heroic_gm_status = StatusLabel()
        self.heroic_mh_status = StatusLabel()
        heroic.add_row("GameMode:", self.heroic_gm_status)
        heroic.add_row("MangoHud:", self.heroic_mh_status)

        self.btn_heroic_gm = QPushButton("Enable GameMode for Heroic")
        self.btn_heroic_gm.clicked.connect(self._toggle_heroic_gamemode)
        heroic._layout.addWidget(self.btn_heroic_gm)

        self.btn_heroic_mh = QPushButton("Enable MangoHud for Heroic")
        self.btn_heroic_mh.clicked.connect(self._toggle_heroic_mangohud)
        heroic._layout.addWidget(self.btn_heroic_mh)

        btn_launch = QPushButton("Launch Heroic")
        btn_launch.clicked.connect(self._launch_heroic)
        heroic._layout.addWidget(btn_launch)

        root.addWidget(heroic)

        # ── Info ──
        info = Card("Live Info")
        self.lbl_distro = StatusLabel("Unknown")
        self.lbl_gm = StatusLabel()
        self.lbl_mh_h = StatusLabel()
        self.lbl_mh_s = StatusLabel()
        info.add_row("Distribution:", self.lbl_distro)
        info.add_row("GameMode:", self.lbl_gm)
        info.add_row("MangoHud (Heroic):", self.lbl_mh_h)
        info.add_row("MangoHud (Steam):", self.lbl_mh_s)
        root.addWidget(info)

        root.addStretch(1)

        # Status bar
        self.status_bar = QStatusBar()
        self.status_bar.setStyleSheet("background: #181825; color: #a6adc8;")
        self.setStatusBar(self.status_bar)
        self.status_bar.showMessage("Ready")

        # Timer
        self._timer = QTimer(self)
        self._timer.timeout.connect(self._refresh_status)
        self._timer.start(REFRESH_MS)
        self._refresh_status()

    # ── Helpers ─────────────────────────────────────────────────
    def _start_worker(self, worker: _BaseWorker) -> None:
        thread = QThread()
        worker.moveToThread(thread)
        worker.finished.connect(thread.quit)
        worker.failed.connect(lambda msg: self.status_bar.showMessage(f"Error: {msg}"))
        thread.started.connect(worker.run)
        thread.start()
        self._active.append((thread, worker))
        worker.finished.connect(lambda: self._active.remove((thread, worker)) if (thread, worker) in self._active else None)

    def _refresh_status(self) -> None:
        w = StatusWorker()
        w.gamemode_status.connect(self.lbl_gm.set_status)
        w.mangohud_heroic_status.connect(self.lbl_mh_h.set_status)
        w.steam_gm_status.connect(self.lbl_mh_s.set_status)
        w.mangohud_steam_status.connect(self.lbl_mh_s.set_status)
        w.distro_name.connect(self.lbl_distro.set_status)
        w.finished.connect(lambda: self.status_bar.showMessage("Ready"))
        w.finished.connect(self._sync_heroic_ui_from_cfg)
        self._start_worker(w)

    def _check_gamemode(self) -> None:
        self.gm_status.set_status("checking")
        cmd = ["pkexec", str(GAMEMODE_SCRIPT)] if shutil.which("pkexec") else ["bash", str(GAMEMODE_SCRIPT)]
        w = InstallWorker("GameMode", cmd)
        w.checked.connect(lambda name, st: self.gm_status.set_status(st))
        self._start_worker(w)

    def _check_mangohud(self) -> None:
        self.mh_status.set_status("checking")
        found = shutil.which("mangohud") is not None
        self.mh_status.set_status("installed" if found else "not installed")

    def _steam_options(self, enable: bool) -> None:
        self.steam_gm_status.set_status("checking")
        w = SteamWorker(enable)
        w.applied.connect(lambda ok, msg: self.steam_gm_status.set_status("applied" if ok else "not applied"))
        self._start_worker(w)

    def _steam_mh_options(self, enable: bool) -> None:
        self.steam_mh_status.set_status("checking")
        w = SteamMangoHudWorker(enable)
        w.applied.connect(lambda ok, msg: self.steam_mh_status.set_status("applied" if ok else "not applied"))
        self._start_worker(w)

    def _sync_heroic_ui_from_cfg(self) -> None:
        gm = False
        mh = False
        try:
            if HEROIC_CFG.exists():
                data = json.loads(HEROIC_CFG.read_text())
                gm = bool(data.get("defaultSettings", {}).get("useGameMode") or data.get("settings", {}).get("useGameMode", False))
                mh = bool(data.get("defaultSettings", {}).get("enableMangoHud") or data.get("settings", {}).get("enableMangoHud", False))
        except Exception:
            pass

        self.btn_heroic_gm.setChecked(gm)
        self.btn_heroic_gm.setText("Disable GameMode for Heroic" if gm else "Enable GameMode for Heroic")
        self.heroic_gm_status.set_status("on" if gm else "off")

        self.btn_heroic_mh.setChecked(mh)
        self.btn_heroic_mh.setText("Disable MangoHud for Heroic" if mh else "Enable MangoHud for Heroic")
        self.heroic_mh_status.set_status("on" if mh else "off")

    def _heroic_apply_visual_state(self, is_on: bool) -> None:
        text = "Disable GameMode for Heroic" if is_on else "Enable GameMode for Heroic"
        color = "#a6e3a1" if is_on else "#f38ba8"
        self.btn_heroic_gm.setText(text)
        self.btn_heroic_gm.setChecked(is_on)
        self.btn_heroic_gm.setStyleSheet(f"color: {color};")
        self.heroic_gm_status.set_status("on" if is_on else "off")

    def _heroic_current(self, key: str) -> bool:
        try:
            if HEROIC_CFG.exists():
                data = json.loads(HEROIC_CFG.read_text())
                for sec in ("defaultSettings", "settings"):
                    if sec in data and key in data[sec]:
                        return bool(data[sec][key])
        except Exception:
            pass
        return False

    def _toggle_heroic_gamemode(self) -> None:
        current = self._heroic_current("useGameMode")
        wanted = not current
        self._heroic_apply_visual_state(wanted)
        self.btn_heroic_gm.setEnabled(False)
        self.status_bar.showMessage("Toggling GameMode for Heroic...")
        w = ToggleWorker("useGameMode", wanted)
        w.toggled.connect(self._on_heroic_gm)
        self._start_worker(w)

    def _on_heroic_gm(self, ok: bool, msg: str) -> None:
        self.btn_heroic_gm.setEnabled(True)
        if ok:
            self._sync_heroic_ui_from_cfg()
        else:
            self._sync_heroic_ui_from_cfg()
            from PySide6.QtWidgets import QMessageBox
            QMessageBox.warning(self, "Error", msg)
        self.status_bar.showMessage(msg)

    def _heroic_apply_mh_visual_state(self, is_on: bool) -> None:
        text = "Disable MangoHud for Heroic" if is_on else "Enable MangoHud for Heroic"
        color = "#a6e3a1" if is_on else "#f38ba8"
        self.btn_heroic_mh.setText(text)
        self.btn_heroic_mh.setStyleSheet(f"color: {color};")
        self.heroic_mh_status.set_status("on" if is_on else "off")

    def _toggle_heroic_mangohud(self) -> None:
        current = self._heroic_current("enableMangoHud")
        wanted = not current
        self._heroic_apply_mh_visual_state(wanted)
        self.btn_heroic_mh.setEnabled(False)
        self.status_bar.showMessage("Toggling MangoHud for Heroic...")
        w = ToggleWorker("enableMangoHud", wanted)
        w.toggled.connect(self._on_heroic_mh)
        self._start_worker(w)

    def _on_heroic_mh(self, ok: bool, msg: str) -> None:
        self.btn_heroic_mh.setEnabled(True)
        if ok:
            self._sync_heroic_ui_from_cfg()
        else:
            self._sync_heroic_ui_from_cfg()
            from PySide6.QtWidgets import QMessageBox
            QMessageBox.warning(self, "Error", msg)
        self.status_bar.showMessage(msg)

    def _launch_heroic(self) -> None:
        try:
            subprocess.Popen(["heroic"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
            self.status_bar.showMessage("Heroic launched")
        except Exception as e:
            from PySide6.QtWidgets import QMessageBox
            QMessageBox.critical(self, "Error", f"Failed to launch Heroic:\n{e}")


def main() -> int:
    app = QApplication(sys.argv)
    _apply_dark_palette(app)
    if ICON_PATH.exists():
        app.setWindowIcon(QIcon(str(ICON_PATH)))

    window = MainWindow()
    window.show()
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
