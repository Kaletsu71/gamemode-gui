.PHONY: install uninstall clean run rpm build release

PREFIX   ?= /usr/local
BINDIR    = $(PREFIX)/bin
DATADIR   = $(PREFIX)/share
DESKTOPDIR= $(DATADIR)/applications
ICONDIR   = $(DATADIR)/icons/hicolor/scalable/apps

build:
	cargo build

release:
	cargo build --release

install: release
	install -Dm755 target/release/gamemode $(BINDIR)/gamemode
	install -Dm644 gamemode-manager.desktop $(DESKTOPDIR)/gamemode-manager.desktop
	install -Dm644 assets/gamemode-manager.svg $(ICONDIR)/gamemode-manager.svg
	update-desktop-database $(DESKTOPDIR) || true
	gtk-update-icon-cache -f -t $(DATADIR)/icons/hicolor || true

uninstall:
	rm -f $(BINDIR)/gamemode
	rm -f $(DESKTOPDIR)/gamemode-manager.desktop
	rm -f $(ICONDIR)/gamemode-manager.svg

rpm:
	bash packaging/build-rpm.sh

clean:
	cargo clean
	rm -rf dist build *.egg-info __pycache__ *.pyc src/__pycache__

run:
	cargo run
