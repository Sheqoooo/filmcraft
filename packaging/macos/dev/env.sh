# shellcheck shell=bash
# Finder launches do not inherit a terminal's PATH.
DEV_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
export PATH="$DEV_ROOT/.dev-tools/cargo/bin:$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
if [ -x "$DEV_ROOT/.dev-tools/cargo/bin/rustup" ]; then
  export CARGO_HOME="$DEV_ROOT/.dev-tools/cargo"
  export RUSTUP_HOME="$DEV_ROOT/.dev-tools/rustup"
fi
export CARGO_TARGET_DIR="$DEV_ROOT/target"
export MACOSX_DEPLOYMENT_TARGET=11.0
