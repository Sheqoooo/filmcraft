#!/usr/bin/env bash
# One-time setup, also callable by double-clicking FilmCraft einrichten.command.
set -euo pipefail
# shellcheck source=env.sh
. "$(dirname "${BASH_SOURCE[0]}")/env.sh"
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo 'An Apple Silicon Mac is required.' >&2; exit 1;
fi
xcrun --find clang >/dev/null || {
  echo 'Install the Xcode Command Line Tools, then retry.' >&2; exit 1;
}
if ! command -v cargo >/dev/null; then
  mkdir -p "$DEV_ROOT/.dev-tools"
  INSTALLER="$DEV_ROOT/.dev-tools/rustup-init.sh"
  curl --proto '=https' --tlsv1.2 -fL https://sh.rustup.rs -o "$INSTALLER"
  CARGO_HOME="$DEV_ROOT/.dev-tools/cargo" RUSTUP_HOME="$DEV_ROOT/.dev-tools/rustup" \
    sh "$INSTALLER" -y --no-modify-path --profile minimal --default-toolchain 1.95.0
  . "$DEV_ROOT/packaging/macos/dev/env.sh"
fi
/bin/bash "$DEV_ROOT/packaging/macos/dev/install-helper.sh"
/bin/bash "$DEV_ROOT/packaging/macos/dev/build.sh"
open -R "$DEV_ROOT/FilmCraft Dev.app"
