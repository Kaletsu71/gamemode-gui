#!/bin/bash
# Rakentaa RPM-paketin gamemode-managerista (Rust-versio).
# Käyttö: bash packaging/build-rpm.sh
set -euo pipefail

VERSION=1.0.0
PKGNAME="gamemode-manager-${VERSION}"
SPECDIR="${HOME}/rpmbuild/SPECS"
SOURCEDIR="${HOME}/rpmbuild/SOURCES"
RPMDIR="${HOME}/rpmbuild/RPMS/x86_64"

# Varmista rpmbuild-hakemistorakenne
mkdir -p "${SPECDIR}" "${SOURCEDIR}" "${HOME}/rpmbuild/BUILD" \
         "${HOME}/rpmbuild/BUILDROOT" "${HOME}/rpmbuild/RPMS" \
         "${HOME}/rpmbuild/SRPMS"

# Generoi Cargo.lock jos puuttuu (tarvitaan --locked buildiin)
if [ ! -f Cargo.lock ]; then
    echo "Generoidaan Cargo.lock..."
    cargo generate-lockfile
fi

# Luo lähdekooditarpalli (sisältää Cargo.lock)
echo "Luodaan lähdetarpalli ${PKGNAME}.tar.gz ..."
git archive --format=tar --prefix="${PKGNAME}/" HEAD | \
    tar --append --file=- Cargo.lock | \
    gzip > "${SOURCEDIR}/${PKGNAME}.tar.gz" || \
    tar -czf "${SOURCEDIR}/${PKGNAME}.tar.gz" \
        --transform "s|^|${PKGNAME}/|" \
        --exclude='.git' \
        --exclude='target' \
        .

# Kopioi spec-tiedosto
cp packaging/gamemode-manager.spec "${SPECDIR}/gamemode-manager.spec"

# Rakenna RPM
echo "Rakennetaan RPM (tämä kestää hetken, Cargo lataa riippuvuudet)..."
rpmbuild -ba "${SPECDIR}/gamemode-manager.spec"

echo ""
echo "Valmis! RPM löytyy:"
ls -lh "${RPMDIR}"/gamemode-manager-*.rpm 2>/dev/null || \
    echo "  ${RPMDIR}/"
echo ""
echo "Asenna komennolla:"
echo "  sudo zypper install ${RPMDIR}/gamemode-manager-${VERSION}-0.x86_64.rpm"
