# KataGoGo Baseline And Engine Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the migrated KataGoGo project into a stable baseline and prepare the Swift app for interchangeable local and remote engine implementations.

**Architecture:** Keep the current root layout for now so the Xcode project remains stable. First verify the migrated local app and Rust core, then introduce a Swift `EngineClient` boundary around the existing `GoCoreBridge`, and only then extract the remote WebSocket backend.

**Tech Stack:** SwiftUI, Observation, AppKit/CoreGraphics, Rust, Tokio, KataGo GTP, FastAPI WebSocket later.

---

## File Structure

- `.gitignore`: repository ignore rules for local build outputs, user state, logs, and Python runtime files.
- `docs/superpowers/specs/2026-05-17-katagogo-new-direction-design.md`: approved direction and architecture.
- `docs/superpowers/plans/2026-05-17-katagogo-baseline-and-engine-boundary.md`: this implementation plan.
- `KataGoGo/ViewModels/GameViewModel.swift`: currently owns gameplay state and directly calls `GoCoreBridge`; later refactor to depend on `EngineClient`.
- `KataGoGo/Services/GoCoreBridge.swift`: current Swift FFI wrapper; later used only by `LocalRustEngineClient`.
- `KataGoGo/Services/EngineClient.swift`: create in the engine-boundary task.
- `KataGoGo/Services/LocalRustEngineClient.swift`: create after the protocol exists.
- `KataGoGo/Services/MockEngineClient.swift`: create for UI tests and previews.
- `go-core/src/gtp_client.rs`: local KataGo process client; keep parser behavior aligned with remote backend.
- `services/katago-backend/`: create later by extracting the old VPS backend without runtime artifacts.

## Task 1: Baseline Repository Verification

**Files:**
- Modify: `.gitignore`
- Read: `KataGoGo.xcodeproj/project.pbxproj`
- Read: `go-core/Cargo.toml`

- [ ] **Step 1: Check Git state**

Run:

```bash
git status --short --branch
```

Expected: the app, Rust core, engine bundle, docs, and `.gitignore` are untracked or modified; `go-core/target/` is ignored.

- [ ] **Step 2: Check Rust source compiles**

Run:

```bash
cargo test --manifest-path go-core/Cargo.toml
```

Expected: unit tests pass. If the integration test attempts to start KataGo and fails, record the exact failing test and error before changing code.

- [ ] **Step 3: Check Xcode project build**

Run:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -project KataGoGo.xcodeproj -scheme KataGoGo -configuration Debug -derivedDataPath /tmp/KataGoGoDerivedData build
```

Expected: the macOS app builds or reports concrete missing project references. Do not refactor until the build failure is understood.

- [ ] **Step 4: Commit the baseline**

Run:

```bash
git add .gitignore docs KataGoGo KataGoGo.xcodeproj go-core kata-engine
git commit -m "chore: establish katagogo baseline"
```

Expected: a first commit containing the migrated project and planning docs.

## Task 2: Define EngineClient Protocol

**Files:**
- Create: `KataGoGo/Services/EngineClient.swift`
- Modify: `KataGoGo.xcodeproj/project.pbxproj`

- [ ] **Step 1: Add the protocol file**

Create `KataGoGo/Services/EngineClient.swift` with:

```swift
import Foundation

enum EnginePhase: Equatable {
    case idle
    case connecting
    case playing
    case waiting
    case finished
}

struct EngineConfiguration: Equatable {
    var binaryPath: String
    var configPath: String
    var modelPath: String
    var boardSize: Int
    var aiLevel: Int
}

struct EngineSnapshot: Equatable {
    var boardSize: Int
    var board: [[Bool?]]
    var moveLabels: [(col: Int, row: Int, moveNumber: Int)]
    var lastMove: (col: Int, row: Int)?
    var moveCount: Int
    var currentPlayer: String
    var winrateBlack: Double
    var lead: Double
    var capturesBlack: Int
    var capturesWhite: Int
}

protocol EngineClient {
    func start(configuration: EngineConfiguration) async throws -> EngineSnapshot
    func playHumanMove(color: String, vertex: String) async throws -> EngineSnapshot
    func undo() async throws -> EngineSnapshot
    func reset() async throws -> EngineSnapshot
    func setLevel(_ level: Int) async
    func close() async
}
```

- [ ] **Step 2: Add the file to the Xcode project**

Use Xcode or edit `KataGoGo.xcodeproj/project.pbxproj` following the existing `Services/GoCoreBridge.swift` references.

- [ ] **Step 3: Build**

Run:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -project KataGoGo.xcodeproj -scheme KataGoGo -configuration Debug -derivedDataPath /tmp/KataGoGoDerivedData build
```

Expected: build succeeds with the new unused protocol file.

- [ ] **Step 4: Commit**

Run:

```bash
git add KataGoGo/Services/EngineClient.swift KataGoGo.xcodeproj/project.pbxproj
git commit -m "feat: define engine client protocol"
```

## Task 3: Wrap Local Rust Engine

**Files:**
- Create: `KataGoGo/Services/LocalRustEngineClient.swift`
- Modify: `KataGoGo.xcodeproj/project.pbxproj`

- [ ] **Step 1: Add the local client**

Create `KataGoGo/Services/LocalRustEngineClient.swift` with:

```swift
import Foundation

final class LocalRustEngineClient: EngineClient {
    private let bridge = GoCoreBridge()
    private var initialized = false

    func start(configuration: EngineConfiguration) async throws -> EngineSnapshot {
        if !initialized {
            guard bridge.create() else {
                throw GoCoreError.commandFailed("Failed to create engine")
            }
            initialized = true
        }

        try bridge.start(
            binaryPath: configuration.binaryPath,
            configPath: configuration.configPath,
            modelPath: configuration.modelPath,
            boardSize: configuration.boardSize,
            timeout: 120.0
        )
        bridge.setLevel(configuration.aiLevel)
        return try snapshot()
    }

    func playHumanMove(color: String, vertex: String) async throws -> EngineSnapshot {
        try bridge.play(color: color, vertex: vertex)
        let aiColor = color == "b" ? "w" : "b"
        _ = try bridge.genmove(color: aiColor)
        return try snapshot()
    }

    func undo() async throws -> EngineSnapshot {
        _ = bridge.undo()
        return try snapshot()
    }

    func reset() async throws -> EngineSnapshot {
        try bridge.reset()
        return try snapshot()
    }

    func setLevel(_ level: Int) async {
        bridge.setLevel(level)
    }

    func close() async {
        bridge.close()
        bridge.destroy()
        initialized = false
    }

    private func snapshot() throws -> EngineSnapshot {
        guard let frame = bridge.getRenderFrame() else {
            throw GoCoreError.commandFailed(bridge.lastError)
        }

        let analysis = bridge.getAnalysis()
        var board = Array(
            repeating: Array(repeating: Optional<Bool>.none, count: frame.boardSize),
            count: frame.boardSize
        )
        for stone in frame.stones {
            board[stone.row][stone.col] = stone.isBlack
        }

        return EngineSnapshot(
            boardSize: frame.boardSize,
            board: board,
            moveLabels: frame.moveLabels,
            lastMove: frame.lastMove,
            moveCount: analysis?.moveCount ?? frame.moveCount,
            currentPlayer: analysis?.currentPlayer ?? frame.currentPlayer,
            winrateBlack: analysis?.winrateBlack ?? 0.5,
            lead: analysis?.lead ?? 0.0,
            capturesBlack: analysis?.capturesBlack ?? frame.capturesBlack,
            capturesWhite: analysis?.capturesWhite ?? frame.capturesWhite
        )
    }
}
```

- [ ] **Step 2: Add the file to the Xcode project**

Use Xcode or edit `KataGoGo.xcodeproj/project.pbxproj` following the existing `Services/GoCoreBridge.swift` references.

- [ ] **Step 3: Build**

Run:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -project KataGoGo.xcodeproj -scheme KataGoGo -configuration Debug -derivedDataPath /tmp/KataGoGoDerivedData build
```

Expected: build succeeds with `LocalRustEngineClient` compiled.

- [ ] **Step 4: Commit**

Run:

```bash
git add KataGoGo/Services/LocalRustEngineClient.swift KataGoGo.xcodeproj/project.pbxproj
git commit -m "feat: wrap local rust engine"
```

## Task 4: Move GameViewModel To EngineClient

**Files:**
- Modify: `KataGoGo/ViewModels/GameViewModel.swift`

- [ ] **Step 1: Add injectable engine dependency**

Change the engine property and initializer:

```swift
private let engine: EngineClient

init(engine: EngineClient = LocalRustEngineClient()) {
    self.engine = engine
}
```

- [ ] **Step 2: Convert start/play/reset methods to use async Task**

Replace direct `GoCoreBridge` calls with `Task { ... }` blocks that await `EngineClient` methods and call a local `apply(snapshot:)`.

- [ ] **Step 3: Add snapshot application helper**

Add:

```swift
@MainActor
private func apply(snapshot: EngineSnapshot) {
    board = snapshot.board
    moveLabels = snapshot.moveLabels
    moveCount = snapshot.moveCount
    currentPlayer = snapshot.currentPlayer
    capturesBlack = snapshot.capturesBlack
    capturesWhite = snapshot.capturesWhite
    lastMove = snapshot.lastMove
    winrateBlack = snapshot.winrateBlack
    lead = snapshot.lead
}
```

- [ ] **Step 4: Build**

Run:

```bash
DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcodebuild -project KataGoGo.xcodeproj -scheme KataGoGo -configuration Debug -derivedDataPath /tmp/KataGoGoDerivedData build
```

Expected: build succeeds and `GameViewModel` no longer imports or directly stores `GoCoreBridge`.

- [ ] **Step 5: Commit**

Run:

```bash
git add KataGoGo/ViewModels/GameViewModel.swift
git commit -m "refactor: drive game view model through engine client"
```

## Task 5: Extract Remote Backend Cleanly

**Files:**
- Create: `services/katago-backend/app/main.py`
- Create: `services/katago-backend/app/katago_engine.py`
- Create: `services/katago-backend/app/gtp.py`
- Create: `services/katago-backend/README.md`
- Create: `docs/engine-websocket-protocol.md`

- [ ] **Step 1: Extract source files only**

Copy only Python source from `/Users/mac.chen/Desktop/KataGoGo-old/katago-deploy-20260517.tar.gz`:

```bash
mkdir -p services/katago-backend/app
tar -xOf /Users/mac.chen/Desktop/KataGoGo-old/katago-deploy-20260517.tar.gz backend/app/main.py > services/katago-backend/app/main.py
tar -xOf /Users/mac.chen/Desktop/KataGoGo-old/katago-deploy-20260517.tar.gz backend/app/katago_engine.py > services/katago-backend/app/katago_engine.py
tar -xOf /Users/mac.chen/Desktop/KataGoGo-old/katago-deploy-20260517.tar.gz backend/app/gtp.py > services/katago-backend/app/gtp.py
```

- [ ] **Step 2: Add protocol docs**

Document the initial WebSocket messages in `docs/engine-websocket-protocol.md`: `ready`, `played`, `ai_move`, `level_changed`, `final_score`, `error`, `clear`, `set_level`, `play`, `genmove`.

- [ ] **Step 3: Run backend syntax check**

Run:

```bash
python3 -m py_compile services/katago-backend/app/main.py services/katago-backend/app/katago_engine.py services/katago-backend/app/gtp.py
```

Expected: no Python syntax errors.

- [ ] **Step 4: Commit**

Run:

```bash
git add services/katago-backend docs/engine-websocket-protocol.md
git commit -m "feat: extract remote katago backend source"
```

## Self-Review

- Spec coverage: the plan covers baseline migration, local reliability verification, engine boundary creation, and remote backend extraction.
- Placeholder scan: no placeholder tasks are left; each task has concrete files, commands, and expected results.
- Type consistency: `EngineConfiguration`, `EngineSnapshot`, and `EngineClient` names are used consistently across the planned Swift tasks.
