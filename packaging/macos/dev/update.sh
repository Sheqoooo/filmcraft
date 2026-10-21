#!/usr/bin/env bash
# GUI helper backend. Never discards local edits or updates from upstream.
set -euo pipefail
# shellcheck source=env.sh
. "$(dirname "${BASH_SOURCE[0]}")/env.sh"
cd "$DEV_ROOT"
case "${1:-build}" in
  build) ;;
  update)
    [ "$(git remote get-url origin)" = 'https://github.com/Sheqoooo/filmcraft.git' ] || {
      echo 'Update refused: origin is not Sheqoooo/filmcraft.' >&2; exit 1;
    }
    [ "$(git branch --show-current)" = main ] || {
      echo 'Update refused: checkout is not on main. Rebuild local changes instead.' >&2; exit 1;
    }
    [ -z "$(git status --porcelain --untracked-files=normal)" ] || {
      echo 'Local changes exist. Use “Lokal neu bauen”; commit/review changes before downloading updates.' >&2; exit 1;
    }
    git fetch origin main
    git merge --ff-only origin/main
    ;;
  *) echo 'Usage: update.sh [build|update]' >&2; exit 2 ;;
esac
exec /bin/bash "$DEV_ROOT/packaging/macos/dev/build.sh"
