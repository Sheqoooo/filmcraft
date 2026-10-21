#!/usr/bin/env bash
# Optimized, incremental Apple Silicon developer bundle; no signing account required.
# Usage: build.sh [--skip-build]. Output: <checkout>/FilmCraft Dev.app
set -euo pipefail
# shellcheck source=env.sh
. "$(dirname "${BASH_SOURCE[0]}")/env.sh"
# Reuse the release pipeline's version/build metadata and font-licence copying.
# shellcheck source=../../env.sh
. "$DEV_ROOT/packaging/env.sh"
SKIP_BUILD=0
case "${1:-}" in
  '') ;;
  --skip-build) SKIP_BUILD=1 ;;
  *) echo "Usage: $0 [--skip-build]" >&2; exit 2 ;;
esac
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo 'This developer bundle requires an Apple Silicon Mac.' >&2; exit 1;
fi
command -v cargo >/dev/null || { echo 'Rust is missing. Install Rust >= 1.95, then retry.' >&2; exit 1; }
xcrun --find clang >/dev/null
WORK="$DEV_ROOT/dist/dev"
mkdir -p "$WORK"
LOCK="$WORK/build.lock"
mkdir "$LOCK" 2>/dev/null || {
  echo "A build is already running. If it was interrupted, remove $LOCK after checking." >&2; exit 1;
}
STAGE="$(mktemp -d "$WORK/stage.XXXXXX")"
cleanup() { rmdir "$LOCK"; }
trap cleanup EXIT
cd "$DEV_ROOT"
export FILMCRAFT_BUILD_SHA
FILMCRAFT_BUILD_SHA="$(git rev-parse HEAD)"
if [ -n "$(git status --porcelain --untracked-files=normal)" ]; then
  FILMCRAFT_BUILD_SHA="$FILMCRAFT_BUILD_SHA-dirty"
fi
export FILMCRAFT_BUILD_DATE
FILMCRAFT_BUILD_DATE="$(date -u +%Y-%m-%d)"
if [ "$SKIP_BUILD" = 0 ]; then
  cargo build --locked --profile dev-app --target aarch64-apple-darwin -p filmcraft
fi
BINARY="$CARGO_TARGET_DIR/aarch64-apple-darwin/dev-app/filmcraft"
file "$BINARY"
lipo "$BINARY" -verify_arch arm64
APP="$STAGE/FilmCraft Dev.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BINARY" "$APP/Contents/MacOS/FilmCraft-bin"
cp "$DEV_ROOT/assets/app-icon/filmcraft.icns" "$APP/Contents/Resources/FilmCraft.icns"
# The launcher keeps this fork's preferences/recovery separate from the official app.
cat > "$APP/Contents/MacOS/FilmCraft" <<'LAUNCH'
#!/bin/bash
set -eu
APP_DIR="$(cd "$(dirname "$0")" && pwd)"
export FILMCRAFT_DATA_DIR="${FILMCRAFT_DATA_DIR:-$HOME/Library/Application Support/FilmCraft Dev}"
mkdir -p "$FILMCRAFT_DATA_DIR"
exec "$APP_DIR/FilmCraft-bin" "$@"
LAUNCH
chmod +x "$APP/Contents/MacOS/FilmCraft"
SHORT_VERSION="${VERSION%%-*}"
sed -e "s/@VERSION@/$VERSION-dev/g" -e "s/@SHORT_VERSION@/$SHORT_VERSION/g" \
    -e "s/@BUILD_SHA@/$FILMCRAFT_BUILD_SHA/g" \
    -e 's/ai.storyteller.filmcraft<\/string>/com.sheqoooo.filmcraft.dev<\/string>/' \
    "$DEV_ROOT/packaging/macos/Info.plist.in" > "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleDisplayName FilmCraft Dev' "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleName FilmCraft Dev' "$APP/Contents/Info.plist"
# Dev app does not claim ownership of .fcproj files over the official installation.
/usr/libexec/PlistBuddy -c 'Set :CFBundleDocumentTypes:0:LSHandlerRank Alternate' "$APP/Contents/Info.plist"
cp "$DEV_ROOT/LICENSE-MIT" "$DEV_ROOT/LICENSE-APACHE" "$DEV_ROOT/NOTICE" "$DEV_ROOT/ATTRIBUTION.md" "$APP/Contents/Resources/"
cp "$DEV_ROOT/assets/app-icon/LICENSE.txt" "$APP/Contents/Resources/AppIcon-LICENSE.txt"
copy_font_licences "$APP/Contents/Resources"
printf 'APPL????' > "$APP/Contents/PkgInfo"
printf 'Commit: %s\nBuilt: %s\nProfile: dev-app\nTarget: aarch64-apple-darwin\n' \
  "$FILMCRAFT_BUILD_SHA" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$APP/Contents/Resources/DeveloperBuild.txt"
plutil -lint "$APP/Contents/Info.plist"
codesign --force --sign - --timestamp=none "$APP/Contents/MacOS/FilmCraft-bin"
codesign --force --sign - --timestamp=none "$APP"
codesign --verify --strict --deep "$APP"
"$APP/Contents/MacOS/FilmCraft-bin" --version
DEST="$DEV_ROOT/FilmCraft Dev.app"
# Preserve the last working bundle until its replacement is fully built and verified.
PREVIOUS="$WORK/FilmCraft Dev.previous.$(date +%Y%m%d-%H%M%S).$$.app"
if [ -e "$DEST" ]; then mv "$DEST" "$PREVIOUS"; fi
if ! mv "$APP" "$DEST"; then
  [ ! -e "$PREVIOUS" ] || mv "$PREVIOUS" "$DEST"
  echo 'Could not install the new app; the previous version was restored.' >&2
  exit 1
fi
rmdir "$STAGE"
echo "Ready: $DEST"
