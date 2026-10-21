# Local validation — 2026-10-21

Status: local installation completed. The owner subsequently authorized publishing this
development snapshot and its macOS build workflow to Sheqoooo/filmcraft despite the documented
failed/incomplete checks below. The fork policy in AGENTS.md and CLAUDE.md records that standing
authorization for future development snapshots. This is not a stable release or an upstream change.
The workflow is configured to build development artifacts; its actual run status must be checked
on GitHub rather than inferred from the successful local build.

## Verified

- Native optimized `aarch64-apple-darwin` developer app built from this checkout.
- Final rebuild completed, and the previous working app was retained under `dist/dev/`.
- Bundle metadata and ad-hoc code signatures verified successfully.
- App launched through macOS Launch Services; the UI and About dialog were inspected using
  FilmCraft's control channel and screenshots (`target/dev-app-smoke.png`, `target/dev-app-about.png`).
- The official app (`ai.storyteller.filmcraft`) and fork (`com.sheqoooo.filmcraft.dev`) were both
  observed running concurrently. The official app was not closed or modified.
- Fork launcher uses `~/Library/Application Support/FilmCraft Dev` by default.
- Updater compiled as a separate app. Its installed source explicitly says that the normal
  FilmCraft can remain open. The user cancelled its dialogs; a complete rebuild initiated by
  clicking through the GUI was not verified. The same build backend was run successfully.
- Update backend refused to fetch/merge over the local source changes.
- Concurrent build lock refused a second build without replacing the existing app.
- ZIP archive unpacked with executable permissions and valid app signatures preserved.
- Shell syntax, workflow YAML and fork-only workflow guard checked.
- `cargo fmt --check` and workspace Clippy with warnings denied passed.
- `cargo xtask layers` and `cargo xtask assets` passed (69 assets attributed).

## Incomplete / failed checks

The full `cargo xtask ci` gate is **not green**. Its release test build was rerun with LTO off
and 16 code-generation units to avoid expensive repeated LTO for the many test executables;
the tests and release optimization were retained. A separate earlier run also passed release Clippy.

The workspace test run failed at:

`apps/filmcraft-cli/tests/workflows.rs` → `phone_vfr_clip_extract_gain_title_to_h264_mp4`

At exported frame 20, the closest source frame is correctly frame 20, but the measured pixel
mean absolute difference is 19.04, exceeding the test's threshold of 6.0. The CLI workflow test,
CLI implementation, codecs, render/export engine and platform media code are unchanged by this
setup. This is not a verified regression caused by the packaging changes; it is also not proof
that all other export behavior is correct. No test threshold was loosened and the test was not
removed. The run stopped at this failure. Before it stopped, 417 tests passed and eight were
ignored. Remaining workspace tests have not all been run to completion.

The WebAssembly check could not download some missing dependencies because network access
from the sandbox was blocked. This is an incomplete check, not a confirmed source-code failure.
It does not prevent the native app from building or launching.

Logs:

- Final native build: `dist/dev/final-build.log`
- Full quality run: `/private/tmp/filmcraft-quality-fast.log`
- Layers/assets/WebAssembly: `/private/tmp/filmcraft-gate-{layers,assets,wasm}.log`
- Future GUI rebuilds: `dist/dev/build.log`

No Desktop shortcuts were installed.
Start the apps from the Filmcraft project folder. Do not move the checkout without regenerating
the updater through `FilmCraft einrichten.command`.
