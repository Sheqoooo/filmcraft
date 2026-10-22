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

## Developer icon follow-up

The macOS developer app now uses a teal owl with an original geometric DEV badge, both in the
bundle icon and in the embedded runtime Dock icon. The official app and standard artwork remain
unchanged. The two new icon assets are attributed (asset check: 71 assets), and the generation
script reuses the openly licensed original owl; its rendering tools are not runtime dependencies.
A native rebuild, ARM64 app Clippy, formatting, asset checks and bundle signature checks passed.
The icon was inspected visually. An already-running app needs a normal restart to display the
new runtime Dock icon; no user session or Dock process was forcibly closed.

The first automatic macOS ARM64 developer build and packaging lint succeeded on GitHub before
this icon follow-up, and a ZIP artifact was produced:
https://github.com/Sheqoooo/filmcraft/actions/runs/39629631477

## Selected-effect clipboard (2026-10-21)

Copy in Effect Controls now copies the selected effect snapshot instead of a timeline clip.
Paste applies it to compatible selected clips in one undo step; intrinsic effects replace their
matching built-in instance. Parameters, enabled state, masks and animation are retained, with
keyframe offsets relative to the destination source in-point. Timeline Copy/Cut replaces the
effect clipboard, and OS media clipboard handling retains its existing precedence.

Validation: 34 engine clip-operations tests, 18 keyframe UI tests (including the real Transform
header + Cmd+C / target selection + Cmd+V workflow), and 6 OS clipboard UI tests passed.
The additional engine cases cover snapshot independence, multiple targets, undo/redo, intrinsic
replacement, masks, keyframe timing, linked audio targeting and invalid/locked/incompatible targets.
Scoped engine/UI Clippy with warnings denied, formatting, the native ARM64 developer build and
strict bundle signature verification passed. No running app was closed or user project edited.
The earlier workspace-wide export-test failure and incomplete wasm gate remain as described above;
these unrelated full gates were not rerun for this change.

## Motion / parameter clipboard follow-up (2026-10-22)

The user reported clip duplication after restarting the previous build. An additional German
Motion-header test using the macOS command inbox passed even before this follow-up, so the reported
runtime failure was not reproduced headlessly. Effect/parameter clicks now explicitly focus
Effect Controls. Copy there refuses an absent/stale selection and clears old internal clip data
instead of silently falling back to copying a clip. Successful effect/parameter Copy names its
content in the status bar.

Parameter labels now select individual parameters. Copy/Paste transfers that parameter and its
animation while retaining other target settings. Motion and other intrinsic effects are updated
in place; a missing standard effect is initialized with defaults. Invalid parameter selection
leaves the existing selection unchanged. The Program transform-handle regression now expects the
more precise parameter selection while retaining its complete gesture/keyframe/undo checks.

Validation: 35 engine clip-operation tests and 34 UI tests (21 keyframe/clipboard workflow,
6 OS clipboard, 7 transform-handle) passed. Scoped engine/UI Clippy with warnings denied, formatting,
ARM64 developer packaging and strict signature verification passed. A headless rendered image was
inspected for the selected Scale row. The sandbox GPU adapter was unavailable; the first approved
GPU test passed outside the sandbox. A further image test was declined and was not retried.
No user project or running application was changed. Confirmation of the originally reported
runtime failure on the user's workflow is still needed; the prior workspace export/wasm gate
limitations remain unchanged.

## Timeline In/Out mouse grips (2026-10-22)

Added primary-button grips for the existing sequence In/Out range (including marks set with I/O).
The band and boundary guide preview locally; release dispatches one ordinary markers.markIn/Out
command. Escape/focus loss cancels without a history entry. Enabled Timeline snapping targets clip
boundaries, sequence markers, playhead and the other boundary, excluding the dragged mark itself.
Out is drawn at its exclusive edge and committed as the inclusive last frame. Boundaries cannot
cross; short/one-frame intervals resolve overlapping grip targets by their visible grip centers.
The original generic grip geometry is recorded in ATTRIBUTION.md.

Validation: 27 headless UI tests passed across timeline_marks_ui, timeline_trim_ui,
timeline_middle_pan_ui, timeline_escape_ui and marked_playback_position. Two existing optional
trim screenshot tests remained ignored. New real-pointer tests cover I/O-created marks, snapping,
frame alignment, crossing prevention, undo/redo, unchanged clips/playhead, preview cancellation,
and non-primary buttons. Scoped UI Clippy with warnings denied, formatting, native ARM64 developer
packaging and strict signature verification passed. No native window was opened or user project
modified. Additional GPU image checks were not requested. The earlier workspace-wide export-test
failure and incomplete wasm gate remain documented and were not rerun for this UI-only change.

## Shared monitor follows playback (2026-10-22)

Sequence playback now activates Program when Source and Program share a tab group. Source
playback activates Source in the same arrangement. This includes floating groups and maximized
shared monitors, while preserving keyboard focus (Space from the Timeline still pauses sequence
playback). Separate monitor groups and closed Program panels retain their layout. Tab changes
use the existing shared docking activation transaction; playback never edits project data.

Validation: two new regression cases failed against the previous code (Timeline Space left Source
visible; floating/maximized Program activation was missing). After the fix, all 28 scoped tests
passed across monitor_follow_playback_ui, source_playback_ui, shared_docking_ui, playback_clock_ui
and marked_playback_position. Coverage includes real Space input, pause routing, both output
directions, floating geometry, maximized tabs, keyboard focus and unchanged project/layout data.
UI Clippy with warnings denied, formatting, native ARM64 developer packaging and strict signature
verification passed. No user project or running app was manipulated. The previously documented
workspace-wide export failure and incomplete wasm gate were not rerun for this UI-only change.

## Immediate monitor ruler scrubbing (2026-10-22)

Source's implicit full-media range previously captured an ordinary ruler drag as whole-range
translation, leaving nowhere for that range to move; its stationary click only sought on release.
Plain primary-button presses now seek immediately in Source and Program and continue scrubbing
while held. Range-body translation is retained with Option/Alt-drag; direct In/Out trimming remains
unchanged. Mere handle hover cannot steal an already captured ruler scrub. Scrubbing pauses the
corresponding player, preserves marks/project data, and surfaces source seek errors.

Validation: a new real-pointer regression failed on the previous code at the initial moving press
(Source, implicit full range). After the fix, 37 scoped UI tests passed across source_drag_ui,
source_playback_ui, monitor_geometry, monitor_follow_playback_ui and marked_playback_position.
The tests cover moving presses, forward/backward held drags, crossing trim handles, implicit and
explicit marks in both monitors, playback pause, non-primary buttons, and preserved range-edit
preview/undo behavior using the new Option/Alt modifier. UI Clippy with warnings denied,
formatting, native ARM64 developer packaging and strict bundle signature verification passed.
No running app or user project was manipulated. Prior workspace-wide export/wasm gate limitations
remain documented and were not rerun for this UI-only change.
