#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-run}"
APP_NAME="aerie"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

pkill -x "$APP_NAME" >/dev/null 2>&1 || true

case "$MODE" in
  run|dev)
    npm run tauri dev
    ;;
  --build|build)
    npm run tauri build
    ;;
  --logs|logs)
    npm run tauri dev &
    /usr/bin/log stream --info --style compact --predicate "process == \"$APP_NAME\""
    ;;
  --verify|verify)
    npm run build
    cargo test --manifest-path src-tauri/Cargo.toml
    npm run tauri build
    /usr/bin/open -n src-tauri/target/release/bundle/macos/Aerie.app
    sleep 2
    pgrep -x "$APP_NAME" >/dev/null
    pkill -x "$APP_NAME" >/dev/null 2>&1 || true
    ;;
  *)
    echo "usage: $0 [run|dev|--build|--logs|--verify]" >&2
    exit 2
    ;;
esac
