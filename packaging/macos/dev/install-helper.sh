#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=env.sh
. "$(dirname "${BASH_SOURCE[0]}")/env.sh"
mkdir -p "$DEV_ROOT/dist/dev"
SOURCE="$DEV_ROOT/dist/dev/helper.applescript"
# Embed an AppleScript string literal, escaping both backslashes and quotes in checkout paths.
ESCAPED="$(printf '%s' "$DEV_ROOT" | sed 's/\\/\\\\/g; s/"/\\"/g')"
{
  printf 'on run\n    my updateFilmCraft({"%s"})\nend run\n' "$ESCAPED"
  sed 's/^on run argv$/on updateFilmCraft(argv)/; s/^end run$/end updateFilmCraft/' \
    "$DEV_ROOT/packaging/macos/dev/helper.applescript"
} > "$SOURCE"
STAGE="$(mktemp -d "$DEV_ROOT/dist/dev/helper.XXXXXX")"
osacompile -o "$STAGE/FilmCraft aktualisieren.app" "$SOURCE"
PLIST="$STAGE/FilmCraft aktualisieren.app/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Add :CFBundleIdentifier string com.sheqoooo.filmcraft.dev.updater' "$PLIST"
/usr/libexec/PlistBuddy -c 'Set :CFBundleIconFile FilmCraft' "$PLIST"
/usr/libexec/PlistBuddy -c 'Delete :CFBundleIconName' "$PLIST"
cp "$DEV_ROOT/assets/app-icon/filmcraft.icns" "$STAGE/FilmCraft aktualisieren.app/Contents/Resources/FilmCraft.icns"
# osacompile adds generic privacy descriptions; this helper only runs its build scripts.
for key in NSAppleEventsUsageDescription NSAppleMusicUsageDescription NSCalendarsUsageDescription \
  NSCameraUsageDescription NSContactsUsageDescription NSHomeKitUsageDescription \
  NSMicrophoneUsageDescription NSPhotoLibraryUsageDescription NSRemindersUsageDescription \
  NSSiriUsageDescription NSSystemAdministrationUsageDescription; do
  /usr/libexec/PlistBuddy -c "Delete :$key" "$PLIST"
done
codesign --force --sign - --timestamp=none "$STAGE/FilmCraft aktualisieren.app"
codesign --verify --strict --deep "$STAGE/FilmCraft aktualisieren.app"
HELPER="$DEV_ROOT/FilmCraft aktualisieren.app"
if [ -e "$HELPER" ]; then
  mv "$HELPER" "$STAGE/previous.app"
fi
mv "$STAGE/FilmCraft aktualisieren.app" "$HELPER"
echo "Ready: $HELPER"
