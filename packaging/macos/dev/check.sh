#!/usr/bin/env bash
# Run every repository quality gate; avoid repeated LTO for the many test executables.
set -euo pipefail
# shellcheck source=env.sh
. "$(dirname "${BASH_SOURCE[0]}")/env.sh"
cd "$DEV_ROOT"
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
exec cargo xtask ci
