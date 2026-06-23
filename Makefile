.PHONY: install uninstall package clean run

PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
DESKTOPDIR = $(DATADIR)/applications
ICONDIR = $(DATADIR)/icons/hicolor/scalable/apps
PYTHON = python3

install:
	$(PYTHON) -m pip install --upgrade .
	install -Dm644 gamemode-manager.desktop $(DESKTOPDIR)/gamemode-manager.desktop
	install -Dm644 assets/gamemode-manager.svg $(ICONDIR)/gamemode-manager.svg
	update-desktop-database $(DESKTOPDIR) || true

uninstall:
	rm -f $(BINDIR)/gamemode-manager
	rm -f $(DESKTOPDIR)/gamemode-manager.desktop
	rm -f $(ICONDIR)/gamemode-manager.svg
	$(PYTHON) -m pip uninstall -y gamemode-manager || true

package:
	rm -rf dist build *.egg-info
	$(PYTHON) -m build

clean:
	rm -rf dist build *.egg-info __pycache__ *.pyc

run:
	cd /home/kale/Documents/gamemode-gui && $(PYTHON) src/gamemode_manager.py
