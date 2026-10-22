#!/usr/bin/env bash
# Pinned, unmodified MIT/Apache-2.0 EffectCraft CLI, used only by the developer bundle.
# Download is verified before extraction; no third-party binary enters Git history.
install_effectcraft_helper() {
  local app="$1"
  local version=0.14.0
  local archive="effectcraft-cli-${version}-macos-universal.zip"
  local digest=e32f4c5b735d2573ce5ddbc831ed9c96fdb22f7c56a484f2e0ddd31fc24d84fe
  local cache="$DEV_ROOT/.dev-tools/effectcraft"
  local top="effectcraft-cli-${version}-macos-universal"
  mkdir -p "$cache"
  if [ ! -f "$cache/$archive" ]; then
    curl --fail --location --retry 3 --connect-timeout 30 --max-time 600 \
      "https://github.com/storytold/effectcraft/releases/download/v${version}/${archive}" \
      --output "$cache/$archive.partial"
    mv "$cache/$archive.partial" "$cache/$archive"
  fi
  if [ "$(shasum -a 256 "$cache/$archive" | cut -d ' ' -f 1)" != "$digest" ]; then
    echo 'EffectCraft CLI archive checksum mismatch; refusing to bundle it.' >&2
    return 1
  fi
  mkdir -p "$app/Contents/Helpers" "$app/Contents/Resources/EffectCraft"
  unzip -p "$cache/$archive" "$top/effectcraft-cli" > "$app/Contents/Helpers/effectcraft-cli"
  chmod +x "$app/Contents/Helpers/effectcraft-cli"
  for licence in LICENSE-MIT LICENSE-APACHE README.md; do
    unzip -p "$cache/$archive" "$top/$licence" > "$app/Contents/Resources/EffectCraft/$licence"
  done
  printf 'EffectCraft CLI %s, unmodified upstream binary\nSource: https://github.com/storytold/effectcraft\nLicense: MIT OR Apache-2.0\nArchive SHA256: %s\n' \
    "$version" "$digest" > "$app/Contents/Resources/EffectCraft/NOTICE.txt"
  lipo "$app/Contents/Helpers/effectcraft-cli" -verify_arch arm64
  codesign --force --sign - --timestamp=none "$app/Contents/Helpers/effectcraft-cli"
}
