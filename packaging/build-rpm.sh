#!/bin/bash
# Rakentaa RPM-paketin gamemode-managerista (Rust-versio).
# Käyttö: bash packaging/build-rpm.sh
set -euo pipefail

VERSION=1.1.0
PKGNAME="gamemode-manager-${VERSION}"
SPECDIR="${HOME}/rpmbuild/SPECS"
SOURCEDIR="${HOME}/rpmbuild/SOURCES"
RPMDIR="${HOME}/rpmbuild/RPMS/x86_64"
RPM_FILE="${RPMDIR}/gamemode-manager-${VERSION}-0.x86_64.rpm"
GPG_NAME="GameMode Manager Local"

# Varmista rpmbuild-hakemistorakenne
mkdir -p "${SPECDIR}" "${SOURCEDIR}" "${HOME}/rpmbuild/BUILD" \
         "${HOME}/rpmbuild/BUILDROOT" "${HOME}/rpmbuild/RPMS" \
         "${HOME}/rpmbuild/SRPMS"

# Generoi Cargo.lock jos puuttuu
if [ ! -f Cargo.lock ]; then
    echo "Generoidaan Cargo.lock..."
    cargo generate-lockfile
fi

# Luo lähdekooditarpalli
echo "Luodaan lähdetarpalli ${PKGNAME}.tar.gz ..."
tar -czf "${SOURCEDIR}/${PKGNAME}.tar.gz" \
    --transform "s|^\./|${PKGNAME}/|" \
    --exclude='./.git' \
    --exclude='./target' \
    .

# Kopioi spec-tiedosto
cp packaging/gamemode-manager.spec "${SPECDIR}/gamemode-manager.spec"

# Rakenna RPM
echo "Rakennetaan RPM (tämä kestää hetken, Cargo lataa riippuvuudet)..."
rpmbuild -ba "${SPECDIR}/gamemode-manager.spec"

# Allekirjoita jos rpm-sign on asennettu
if command -v rpmsign &>/dev/null; then
    echo "Allekirjoitetaan RPM..."

    # Luo paikallinen GPG-avain jos ei vielä ole
    if ! gpg --list-keys "${GPG_NAME}" &>/dev/null; then
        echo "Luodaan paikallinen GPG-avain..."
        gpg --batch --gen-key <<EOF
%no-protection
Key-Type: RSA
Key-Length: 2048
Name-Real: ${GPG_NAME}
Name-Email: local@localhost
Expire-Date: 0
EOF
    fi

    # Vie avain tiedostoon (rpm --import vaatii sudoa, tehdään se kerran asennuksen yhteydessä)
    GPG_KEY_FILE="${HOME}/rpmbuild/gamemode-local.pub"
    gpg --export -a "${GPG_NAME}" > "${GPG_KEY_FILE}"

    # Lisää macros-tiedosto allekirjoitusta varten
    cat > "${HOME}/.rpmmacros" <<EOF
%_gpg_name ${GPG_NAME}
%_gpg_path ${HOME}/.gnupg
%__gpg /usr/bin/gpg
EOF

    rpmsign --addsign "${RPM_FILE}"
    echo "Paketti allekirjoitettu."
else
    echo "Huom: rpm-sign ei ole asennettu, paketti on allekirjoittamaton."
    echo "      Asenna: sudo zypper install rpm-sign"
fi

echo ""
echo "Valmis! RPM löytyy:"
ls -lh "${RPMDIR}"/gamemode-manager-[0-9]*.rpm 2>/dev/null
echo ""
if command -v rpmsign &>/dev/null && [ -f "${HOME}/rpmbuild/gamemode-local.pub" ]; then
    echo "Tuo allekirjoitusavain kerran (poistaa allekirjoitusvaroituksen):"
    echo "  sudo rpm --import ${HOME}/rpmbuild/gamemode-local.pub"
    echo ""
fi
echo "Asenna komennolla:"
echo "  sudo zypper install ${RPM_FILE}"
