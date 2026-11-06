#!/usr/bin/env bash
set -euo pipefail
RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP=bookos-teclas
# instalar-todo.sh recoge esta carpeta junto a las demás apps de BookOS.
SALIDA="$RAIZ/../../Build/$APP"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

cargo build --release --locked -p "$APP" --manifest-path "$RAIZ/Cargo.toml"
cargo build --release --locked --manifest-path "$RAIZ/../teclasd/Cargo.toml" --target-dir "$RAIZ/target/servicio"
rpmbuild -bb "$RAIZ/$APP.spec" \
  --define "_topdir $TMP" --define "raiz $RAIZ" --quiet
mkdir -p "$SALIDA"
# Uno solo: instalar-todo.sh pasa todos a un mismo `rpm -U` y dos versiones
# del mismo paquete en una transacción chocan.
rm -f "$SALIDA"/bookos-teclas-*.rpm
cp "$TMP"/RPMS/*/*.rpm "$SALIDA/"
ls "$SALIDA"/*.rpm
