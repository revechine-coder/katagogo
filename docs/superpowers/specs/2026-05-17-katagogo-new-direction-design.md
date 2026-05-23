# KataGoGo New Direction Design

> **Project:** KataGoGo macOS Go training client
> **Date:** 2026-05-17
> **Status:** Approved direction, ready for implementation planning

## Goal

KataGoGo should become a native macOS Go training client that can use either a local KataGo engine or a remote VPS KataGo service through the same app experience. The first product milestone is a stable local-play loop: launch KataGo, play a human move, receive an AI response, display the board, and show winrate plus score lead.

## Existing Assets

The current repository was initialized from the newer desktop app at `/Users/mac.chen/Desktop/KataGoGo`.

Important retained assets:

- `KataGoGo/`: SwiftUI macOS app with board, sidebar, toolbar, settings, design system, and FFI bridge.
- `go-core/`: Rust shared core with game state, move history, GTP subprocess client, render frame generation, C ABI, and tests.
- `kata-engine/`: local KataGo engine bundle for development.
- `KataGoGo.xcodeproj/`: Xcode project for the macOS app.

The old deployment archive at `/Users/mac.chen/Desktop/KataGoGo-old/katago-deploy-20260517.tar.gz` contains a FastAPI WebSocket backend. That package is useful as a remote-engine reference, but it should be cleaned before becoming a first-class service directory because it includes runtime logs and a Python virtual environment.

## Product Direction

The app should feel like a focused training board, not a generic engine launcher. The user should be able to:

- Start a game quickly with sensible local defaults.
- Play against KataGo at selectable strength levels.
- See the current position, move count, last move, winrate, and score lead.
- Use settings to choose local engine files or a remote VPS endpoint.
- Recover gracefully when the engine path is wrong, the engine crashes, or the remote server is full.
- Later import/export SGF and review move history.

## Architecture

KataGoGo should converge on a three-layer architecture:

```text
SwiftUI App
  -> GameViewModel
  -> EngineClient protocol
       -> LocalRustEngineClient
            -> GoCoreBridge
            -> Rust go-core
            -> local KataGo GTP subprocess
       -> RemoteWebSocketEngineClient
            -> FastAPI WebSocket backend
            -> remote KataGo GTP subprocess
```

The main architectural change is to stop letting `GameViewModel` talk directly to `GoCoreBridge`. Instead, the app should depend on an `EngineClient` interface. The local Rust engine and remote WebSocket service then become interchangeable implementations.

## Local Engine

The local engine remains the default and should be made reliable before remote mode receives major investment.

Responsibilities:

- Own KataGo process startup and shutdown.
- Validate binary, config, and model paths before starting.
- Send GTP commands in order.
- Parse AI move, black winrate, and score lead.
- Expose a flat render frame to Swift.
- Preserve enough state for undo, reset, and later SGF export.

Near-term improvements:

- Replace direct `GameViewModel -> GoCoreBridge` dependency with `LocalRustEngineClient`.
- Make GTP parsing testable and consistent with the remote backend.
- Fix undo semantics so the KataGo process and app board cannot diverge.
- Tighten FFI string ownership for `go_core_last_error`.

## Remote Engine

Remote mode should reuse the VPS FastAPI WebSocket idea, but with a cleaned package and explicit protocol.

Initial message types:

- Server to client: `ready`, `played`, `ai_move`, `level_changed`, `final_score`, `error`.
- Client to server: `clear`, `set_level`, `play`, `genmove`, `final_score`.

The macOS app should treat remote mode as another `EngineClient`. It should not contain remote-specific game rules in the UI layer.

Remote-specific behavior:

- Surface capacity errors clearly when the VPS has no available KataGo slot.
- Keep a mock engine option for backend smoke tests.
- Keep engine file paths and concurrency limits in environment variables.
- Package the backend without `venv`, `__pycache__`, or runtime logs.

## Data Flow

For both local and remote engines, the app should follow the same user flow:

1. User clicks an intersection.
2. `GameViewModel` validates app phase and board occupancy.
3. `GameViewModel` calls `EngineClient.playHumanMove`.
4. The engine records the human move, requests the AI move, and returns an updated snapshot.
5. `GameViewModel` publishes the board, labels, last move, move count, winrate, lead, and phase.
6. SwiftUI redraws from published state.

The UI should not implement Go rules beyond basic interaction guards.

## Repository Shape

For the current milestone, keep the existing root layout so Xcode paths remain stable:

```text
KataGoGo.xcodeproj/
KataGoGo/
go-core/
kata-engine/
docs/
```

A later cleanup may move toward:

```text
apps/macos/
crates/go-core/
services/katago-backend/
engine/
docs/
```

That structural move should wait until the local app builds and runs from the current repository.

## Milestones

### Milestone 1: Repository Baseline

- Import the newer desktop app into the clean Git repository.
- Add ignore rules for local build outputs and user state.
- Add this design document and a concrete implementation plan.
- Verify Rust tests at least compile or report concrete blockers.

### Milestone 2: Local Play Reliability

- Confirm the macOS app builds from this repository.
- Confirm the local KataGo engine starts using `kata-engine/`.
- Confirm one human move and one AI response update the board.
- Improve visible error messages for missing files or engine failure.

### Milestone 3: EngineClient Boundary

- Add an `EngineClient` protocol in Swift.
- Move current local FFI calls behind `LocalRustEngineClient`.
- Keep `GameViewModel` focused on app state and workflow.
- Add a `MockEngineClient` for UI tests and previews.

### Milestone 4: Remote Engine

- Extract the VPS backend into a clean `services/katago-backend/` directory.
- Define the WebSocket protocol in docs.
- Add `RemoteWebSocketEngineClient`.
- Add settings for local vs remote mode.

### Milestone 5: Training Features

- Add SGF export.
- Add review timeline.
- Add pass, final score, and stronger undo support.
- Add richer analysis display once the engine boundary is stable.

## Risks

- FFI lifecycle bugs can crash the app if string ownership or global state is mishandled.
- KataGo subprocess state can diverge from app state if undo/reset are not synchronized.
- Remote mode can feel unreliable unless capacity and disconnect errors are explicit.
- Large generated artifacts such as Rust `target/`, Python `venv/`, and logs must stay out of Git.

## Acceptance Criteria For The Next Work Session

- The repository contains the migrated app, Rust core, engine files, and docs.
- Build outputs and user state are ignored.
- A plan exists for implementing the local reliability and engine boundary work.
- Verification commands have been run and their results recorded in the final handoff.
