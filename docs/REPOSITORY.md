# Repository Guide

This document explains the main folders in the KataGoGo repository and what should usually be edited there.

## Application

`KataGoGo/` contains the macOS SwiftUI app.

- `KataGoGoApp.swift`: app entry point
- `ContentView.swift`: primary three-column workspace layout
- `DesignSystem.swift`: shared colors, spacing, panel styles, and UI tokens
- `AppState.swift`: persisted app settings such as engine paths, handicap, and sound preset
- `ViewModels/GameViewModel.swift`: game state, engine orchestration, save/load, review mode, and UI-facing state

## Views

`KataGoGo/Views/` contains the main UI surfaces.

- `Board/`: AppKit-backed board rendering and board interaction
- `MiniMap/`: move overview, timeline, label/coordinate controls
- `Sidebar/`: analysis and game library panels
- `Toolbar/`: game controls, level picker, engine settings, sound picker

## Services

`KataGoGo/Services/` contains app service boundaries.

- `GoCoreBridge.swift`: Swift bridge to the Rust static library
- `GameStore.swift`: JSON save/load/delete/list for games
- `FinalScoreService.swift`: final score handling
- `StoneSoundService.swift`: synthesized stone click sounds

## Native Engine Bridge

`go-core/` is a Rust library that wraps Go/KataGo engine operations and exposes a C-compatible header/library for the Swift app.

Important files:

- `go-core/src/ffi.rs`
- `go-core/src/gtp_client.rs`
- `go-core/src/game_state.rs`
- `go-core/src/go_core.h`

Build outputs are copied into:

- `KataGoGo/SharedCore/libgo_core.a`
- `KataGoGo/SharedCore/go_core.h`

## KataGo Resources

`kata-engine/` contains bundled local engine resources:

- KataGo executable builds
- `gtp.cfg`
- neural net model files

Large model files are tracked with Git LFS. Do not replace these files without checking LFS status.

## Vendored KataGo Source

`kata-go-src/` is a source reference for KataGo itself. Treat it as vendored upstream code. Prefer changing app integration code in `go-core/` or `KataGoGo/` unless the task specifically requires changing KataGo source.

## Saved Games

`games/` contains local saved game JSON files and `_index.json`. These files are useful for app testing and demo state, but avoid committing private or accidental game records unless they are intentional fixtures.

## Releases

`releases/` contains packaged app artifacts. Large release files are tracked with Git LFS.

The main release helper is:

```sh
scripts/release-arm64.sh
```

It builds an Apple Silicon package targeting macOS 12 or later.

## Documentation

- `README.md`: GitHub project overview
- `DESIGN.md`: product and UI design system guidance
- `docs/GITHUB_SETUP.md`: GitHub, Git LFS, and publishing notes
- `docs/superpowers/`: planning/spec artifacts from prior development sessions

## Local Files

`.DS_Store`, Xcode user state, Rust `target/`, Python environments, logs, and temporary files should stay untracked. See `.gitignore`.
