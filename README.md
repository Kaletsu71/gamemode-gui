# GameMode Manager

Qt6/PySide6 GUI GameMode + MangoHud + Steam + Heroic.

## Asennus

### Vaatimukset
- Python 3.10+
- PySide6
- gamemoded (GameMode)
- mangohud
- Steam (valinnainen)
- Heroic (valinnainen)

### Asennus venv:llä (ilman sudoa)

```bash
cd gamemode-gui
python3 -m venv .venv
source .venv/bin/activate
pip install .
gamemode-manager
```

### Suora käynnistys ilman asennusta

```bash
cd gamemode-gui
python3 src/gamemode_manager.py
```

### Ohitus .desktop-tiedostolla

```bash
gtk-launch gamemode-manager
```

## Käyttö

- **GameMode toggle**: päälle/pois Heroicille ja Steamille
- **MangoHud toggle**: päälle/pois Heroicille ja Steamille
- **Status-näyttö**: näyttää GameMode/MangoHud tilan reaaliajassa
- **Live Info**: distro, GM/MH-tila

### Steam

- Add GameMode launch options - lisää `gamemoderun %command%` kaikkiin Steam-peleihin
- Add MangoHud launch options - lisää `mangohud %command%` kaikkiin Steam-peleihin

### Heroic

Peli- ja globaali asetuspainikkeet tallentuvat Heroicin config.json ja GamesConfig/*.json -tiedostoihin.

## Kääntäminen

```bash
make run
make package
make clean
```

## Komentoja

- `make run` - käynnistä ilman asennusta
- `make install` - asenna järjestelmään (vaatii sudon)
- `make uninstall` - poista asennus
- `make package` - tee Python-paketti
