# GameMode Manager

Rust + egui -pohjainen graafinen työkalu Linux-pelaamisen suorituskykytyökalujen hallintaan.

## Ominaisuudet

- GameMode ja MangoHud asennuksen tarkistus ja asennus
- Steam-käynnistysasetusten hallinta kaikille peleille (sulkee ja käynnistää Steamin automaattisesti)
- Heroic Games Launcher -integraatio (GameMode / MangoHud toggle)
- Live-tilanäyttö (distro, daemon-tila, asetukset)
- Catppuccin Mocha -tumma teema
- Yksi itsellinen binääri — ei Python- tai Qt-riippuvuuksia

---

## Asennus (RPM — openSUSE / Fedora)

### Lataa valmis paketti

```bash
# Kloonaa repo
git clone https://github.com/Kaletsu71/gamemode-gui.git
cd gamemode-gui
```

Valmis RPM löytyy `releases/`-kansiosta:

```
releases/gamemode-manager-1.0.0.1-0.x86_64.rpm
```

### Asenna

```bash
sudo zypper install releases/gamemode-manager-1.0.0.1-0.x86_64.rpm
```

tai Fedoralla:

```bash
sudo dnf install releases/gamemode-manager-1.0.0.1-0.x86_64.rpm
```

### Käynnistä

Sovellus löytyy sovellusvalikosta nimellä **GameMode Manager**, tai komentoriviltä:

```bash
gamemode
```

---

## Käyttö

### Asennus-kortti

| Painike | Toiminto |
|---------|----------|
| ⬇ Asenna GameMode | Asentaa `gamemode`-paketin (vaatii salasanan) |
| ↺ Tarkista GameMode | Tarkistaa onko `gamemoded` asennettu |
| ⬇ Asenna MangoHud | Asentaa `mangohud`-paketin (vaatii salasanan) |
| ↺ Tarkista MangoHud | Tarkistaa onko `mangohud` asennettu |

### Steam-integraatio

| Painike | Toiminto |
|---------|----------|
| ➕ Lisää GameMode | Lisää `gamemoderun %command%` kaikkiin Steam-peleihin |
| ➖ Poista GameMode | Poistaa `gamemoderun` kaikkien Steam-pelien käynnistysasetuksista |
| ➕ Lisää MangoHud | Lisää `mangohud %command%` kaikkiin Steam-peleihin |
| ➖ Poista MangoHud | Poistaa `mangohud` kaikkien Steam-pelien käynnistysasetuksista |

> **Huom:** Steam suljetaan ja käynnistetään automaattisesti uudelleen muutosten jälkeen, jotta VDF-tiedoston muutokset tallentuvat oikein.

### Heroic Games Launcher

| Painike | Toiminto |
|---------|----------|
| ▶ Ota käyttöön GameMode | Kytkee GameModen päälle Heroicissa |
| ⏹ Poista käytöstä GameMode | Kytkee GameModen pois Heroicissa |
| ▶ Ota käyttöön MangoHud | Kytkee MangoHudin päälle Heroicissa |
| ⏹ Poista käytöstä MangoHud | Kytkee MangoHudin pois Heroicissa |
| 🚀 Käynnistä Heroic | Käynnistää Heroic Games Launcherin |

---

## Rakentaminen lähdekoodista

### Vaatimukset

```bash
# openSUSE
sudo zypper install rust cargo pkgconfig libGL-devel libX11-devel libXrandr-devel fontconfig-devel

# Fedora
sudo dnf install rust cargo pkgconfig mesa-libGL-devel libX11-devel libXrandr-devel fontconfig-devel
```

### Käännä ja aja

```bash
cargo build --release
./target/release/gamemode
```

### Asenna järjestelmään

```bash
make install
```

### Rakenna RPM

```bash
make rpm
```

Valmis paketti ilmestyy hakemistoon `~/rpmbuild/RPMS/x86_64/`.

---

## Lisenssi

MIT — katso [LICENSE](LICENSE)
