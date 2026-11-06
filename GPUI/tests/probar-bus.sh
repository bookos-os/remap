#!/usr/bin/env bash
set -euo pipefail
RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ ${1:-} != privado ]]; then
    cargo test --offline --locked --no-run --manifest-path "$RAIZ/Cargo.toml"
    dbus-run-session -- "$0" privado escritorio
    dbus-run-session -- "$0" privado sistema
    exit
fi
export REMAP_MOTOR="$2"
export REMAP_PRUEBA_BASE
REMAP_PRUEBA_BASE="$(mktemp -d /tmp/remap-bus.XXXXXX)"
export DBUS_SYSTEM_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS"
/usr/bin/python3 "$RAIZ/tests/servicio_prueba.py" &
SERVICIO_PRUEBA=$!
trap 'kill "$SERVICIO_PRUEBA" 2>/dev/null || true; rm -rf "$REMAP_PRUEBA_BASE"' EXIT
for _ in {1..100}; do
    [[ -f "$REMAP_PRUEBA_BASE/listo" ]] && break
    sleep 0.05
done
cargo test --offline --locked --manifest-path "$RAIZ/Cargo.toml" contrato_del_motor -- --ignored
