#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MATCHES_FILE="$(mktemp)"
trap 'rm -f "$MATCHES_FILE"' EXIT

grep -RInE \
  'firebase|oauth|cloud run|/api/analyze|/api/explain|serviceWorker|navigator\.sendBeacon|gtag|google-analytics|posthog|sentry' \
  -- src src-tauri public index.html package.json >"$MATCHES_FILE" 2>/dev/null || true

if [[ -s "$MATCHES_FILE" ]]; then
  printf 'Privacy boundary check failed. Review these cloud/upload/telemetry patterns:\n' >&2
  cat "$MATCHES_FILE" >&2
  exit 1
fi

printf 'Privacy boundary check passed: no v1 cloud/upload/telemetry patterns found.\n'
