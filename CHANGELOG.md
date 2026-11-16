# Changelog

Notable changes to Astrolight are recorded here. Version 3.2.0 is the first public GitHub release. Earlier entries describe local development milestones, not previously published GitHub releases.

## [3.2.0] — 2026-11-16

### Added
- Public Rust/Tauri source, build instructions, protocol notes, design references and selected validation results.
- Windows x64 portable release with SHA-256 checksums.
- Remap inspector showing the selected position, layer and binding; a physical-position selector and customized-key list.
- Draft Undo/Redo, keyboard navigation, cross-category search and Vietnamese search aliases.

### Changed
- Redesigned the dark workspace around the keyboard, with adjacent controls and responsive layouts.
- Replaced the animated RGB preview with neutral keycaps in remap mode for clearer labels and less drawing work.
- Separated local draft status, local save actions and device-write review.
- Kept the 3.1 lighting engine, 28 modes and default input-priority behavior.

### Fixed
- Modifier selection no longer carries over unintentionally when switching target keys.
- Remap controls no longer overlap the keycode palette at tested window sizes.
- Clarified left/right modifier positions and macro names in the inspector.

### Verification and limits
- 22 Rust tests passed; 336 JavaScript/Rust effect-parity cases previously matched exactly.
- Offline UI and native release checks covered draft editing, Undo/Redo, search, macros, profile matching, import/export and responsive layouts.
- Wired Lucky65 USB tested at approximately 30 FPS without HID errors in the recorded release check.
- No firmware keymap readback. Physical remap/macro behavior after writing and persistence across unplugging remain unverified. Bluetooth/2.4 GHz and other hardware revisions remain unverified.

## 3.1.0 — 2026-11-14 (development milestone)

### Added
- Twelve lighting modes, bringing the total to 28: plasma, nebula, ocean, comets, fireflies, chevron, diamonds, helix, ribbon, scanner, heartbeat and pinwheel.
- Effect search, category filters and a typing-test field.

### Fixed
- Reduced custom RGB traffic from 120 FPS to a 30 FPS default; retained 15/60 FPS choices.
- Migrated legacy 120 FPS lighting profiles to 30 FPS.
- Added input priority: pause new RGB frames while keys are held and for 180 ms after key events.
- Added inter-packet pacing and idle time after completed frames; removed catch-up bursts after missed deadlines.
- Retained the per-frame DIRECT handshake after a device experiment showed a single handshake only updates one frame.

Initial user testing no longer observed repeated/late-release keys. This is a mitigation for the observed RGB/input interaction, not proof of a specific firmware defect or a universal fix.

## 3.0.0 — 2026-11-14 (development milestone)

### Added
- Sixteen lighting modes, effect browsing and per-key painting.
- Three keymap layers, modifier combinations and a macro editor/recorder.
- Configurable keyboard intro, looping SVG logo, Caps Lock colors and automatic USB connection.
- Foreground app/game lighting profiles, settings import/export and Windows startup support.

This development build used 120 FPS RGB streaming, later superseded by the input-priority fixes in 3.1.0.

## 2.0.0 — 2026-11-14 (development milestone)

### Changed
- Migrated the Electron prototype to Tauri and Rust.
- Moved HID output and animation timing to a dedicated native worker.
- Added a Canvas preview, SVG intro and automatic connection.

[3.2.0]: https://github.com/validkanaheiuser/astrolight/releases/tag/v3.2.0
