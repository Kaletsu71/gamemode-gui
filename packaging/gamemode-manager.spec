#
# spec file for package gamemode-manager
#
# Copyright (c) 2026 Kalervo Konttinen
# License: MIT
#

Name:           gamemode-manager
Version:        1.0.0
Release:        0
Summary:        GUI for managing GameMode, MangoHud, Steam and Heroic
License:        MIT
URL:            https://github.com/Kaletsu71/gamemode-gui
Source0:        gamemode-manager-%{version}.tar.gz
BuildArch:      x86_64

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  desktop-file-utils
BuildRequires:  pkgconfig(gl)
BuildRequires:  pkgconfig(x11)
BuildRequires:  pkgconfig(xrandr)
BuildRequires:  pkgconfig(fontconfig)

# libGL and X11 are always installed on any Linux desktop
Requires:       libGL.so.1

%description
GameMode Manager on Rust+egui-pohjainen graafinen käyttöliittymä Linux-
pelaamisen suorituskykytyökalujen hallintaan.

Ominaisuudet:
- GameMode ja MangoHud -tilojen hallinta globaalisti
- Steam-käynnistysasetusten muokkaus kaikille peleille
- Heroic Games Launcher -integraatio
- Live-tilanäyttö
- Catppuccin Mocha -tumma teema

Sovellus on yksi itsellinen binääri ilman Python- tai Qt-riippuvuuksia.

%prep
%autosetup -n gamemode-manager-%{version}

%build
cargo build --release --locked

%install
install -Dm755 target/release/gamemode \
    %{buildroot}%{_bindir}/gamemode

install -Dm644 gamemode-manager.desktop \
    %{buildroot}%{_datadir}/applications/gamemode-manager.desktop

install -Dm644 assets/gamemode-manager.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/gamemode-manager.svg

desktop-file-validate \
    %{buildroot}%{_datadir}/applications/gamemode-manager.desktop

%post
%desktop_database_post
%icon_theme_cache_post

%postun
%desktop_database_postun
%icon_theme_cache_postun

%files
%license LICENSE
%doc README.md
%{_bindir}/gamemode
%{_datadir}/applications/gamemode-manager.desktop
%{_datadir}/icons/hicolor/scalable/apps/gamemode-manager.svg

%changelog
* Tue Jun 24 2026 Kalervo Konttinen <kalervo.konttinen@gmail.com> - 1.0.0-0
- Rewrite in Rust + egui (Catppuccin Mocha dark theme)
- Single self-contained binary, no Python/Qt runtime required
- Auto-discovery of Steam userdata directory
- Background status checks with mpsc channels
