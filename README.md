# KataGoGo

KataGoGo is a native macOS Go training workspace built around KataGo. It pairs a quiet SwiftUI board interface with a bundled KataGo engine, game saving, replay review, compact AI readouts, and local engine resources for study sessions.

The app is designed as a desktop training desk: the board stays central, side panels stay dense, and engine feedback is visible without turning the UI into a dashboard.

## System Support

Current source build target:

- macOS 26 or later
- Apple Silicon Macs (`arm64`)
- Xcode with the macOS 26 SDK
- Metal-capable local KataGo engine resources

Packaged release script target:

- `scripts/release-arm64.sh` builds an Apple Silicon package with `MACOSX_DEPLOYMENT_TARGET=12.0`
- Target Macs: Apple Silicon Macs running macOS 12 or later

Intel scripts are present for packaging experiments, but the main checked-in Xcode project and bundled app workflow are currently optimized for Apple Silicon.

## Features

- Native SwiftUI macOS interface
- 19x19 Go board with coordinate and move label toggles
- KataGo engine integration through the local Rust `go-core` bridge
- AI winrate, score lead, model, think time, and step readouts
- Game timer and per-move elapsed-time history
- Save/load local games as JSON
- Review mode with move timeline navigation
- Built-in stone sound presets
- Bundled KataGo engine and model resources
- Git LFS tracking for large model and release artifacts

## Repository Layout

```text
KataGoGo/              SwiftUI macOS application source
KataGoGoTests/         Xcode unit tests
go-core/               Rust bridge for Go/KataGo engine interaction
kata-engine/           Bundled KataGo binaries, config, and model files
kata-go-src/           Vendored KataGo source reference
games/                 Local saved-game JSON store
releases/              Packaged app artifacts
scripts/               Release and asset helper scripts
docs/                  Project notes and repository maintenance docs
DESIGN.md              Product and UI design system guide
```

More detail is in [docs/REPOSITORY.md](docs/REPOSITORY.md).

## Git LFS

This repository uses Git LFS for large KataGo model and release files. Install Git LFS before cloning or pulling the project:

```sh
brew install git-lfs
git lfs install
git clone https://github.com/revechine-coder/katagogo.git
cd katagogo
git lfs pull
```

Without LFS, large model files may appear as small pointer files and the bundled engine resources will be incomplete.

## Build From Source

Requirements:

- macOS with Xcode installed
- Rust toolchain for `go-core`
- Git LFS
- Homebrew libraries used by bundled engine packaging: `libzip`, `xz`, `zstd`

Basic Debug build:

```sh
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer \
xcodebuild \
  -project KataGoGo.xcodeproj \
  -scheme KataGoGo \
  -configuration Debug \
  -derivedDataPath /tmp/KataGoGoDerivedData \
  build
```

Apple Silicon release package:

```sh
scripts/release-arm64.sh
```

## Engine Resources

The app expects KataGo resources under `kata-engine/`, including:

- `katago-metal`
- `katago-eigen`
- `gtp.cfg`
- model files such as `kata1-b18c384nbt.bin.gz`

Large model files are tracked with Git LFS. The default packaged model is the smaller `b18c384` model for practical local play.

## Saved Games

Saved games are JSON files under `games/`, with `_index.json` acting as a lightweight manifest. The app records move history, elapsed time, winrate, score lead, final score when available, and engine model metadata.

## Design Notes

UI direction and implementation guidance live in [DESIGN.md](DESIGN.md). In short: keep the board dominant, use teal only for AI/active signals, keep panels compact, and prefer native macOS controls.
