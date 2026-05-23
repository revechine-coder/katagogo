---
version: 1.2
name: KataGoGo-design-system
description: A focused native macOS Go training workspace with Metal GPU engine, game save/load, replay analysis, real-time game timer, and teal AI-state signals.
updated: 2026-05-18
---

# KataGoGo DESIGN.md

## 1. Visual Theme & Atmosphere

KataGoGo should feel like a native macOS Go training desk: calm, exact, and useful during long study sessions. The board is the visual anchor. Supporting panels should feel like light instruments around it, not competing dashboards.

Use a warm off-white app canvas, translucent white panels, a realistic but restrained wooden board, black/white stone contrast, and teal only for AI state, advantage, selection, or active feedback. The interface should be quiet enough for reading positions while still making engine state obvious.

Avoid marketing-page composition, oversized decorative cards, saturated gradients, playful illustrations, or heavy glass effects. This is a professional training tool.

## 2. Color Palette & Roles

| Token | Value | Role |
|---|---:|---|
| `canvas.app` | `#F3F4F3` | Main app background. |
| `surface.base` | `#FBFCFB` | Toolbar and persistent chrome. |
| `surface.panel` | `rgba(255,255,255,0.88)` | Floating analysis and utility panels. |
| `line.hairline` | `rgba(0,0,0,0.09)` | Panel borders and section dividers. |
| `text.primary` | `#1F2424` | Primary labels and metric values. |
| `text.secondary` | `#6E7875` | Panel labels and supportive text. |
| `text.tertiary` | `#949C99` | Disabled controls and low-priority metadata. |
| `signal.ai` | `#1C9C9E` | AI signal, active state, advantage, selected rows. |
| `signal.ai-dark` | `#146B6E` | Icons and high-contrast teal labels. |
| `signal.ai-soft` | `#C7E8E6` | Active row and soft highlights. |
| `status.warning` | `#C97923` | Finished, caution, recoverable issues. |
| `board.wood-light` | `#DBAB6B` | Board gradient highlight. |
| `board.wood-dark` | `#85572E` | Board gradient shadow. |

Teal is a signal color, not a background theme. Use it sparingly for current engine insight, selected board/move states, and active controls.

## 3. Typography Rules

Use the native system type stack. Prefer rounded system text for friendly controls and metrics, monospaced digits for changing values, and standard system text for dense body copy.

Font sizing follows macOS system conventions. Minimum readable size is `.caption` (10pt). `.caption2` (9pt) is not used.

| Role | SwiftUI Style |
|---|---|
| Toolbar controls | `.system(.body, design: .rounded, weight: .medium)` (13pt) |
| Panel title | `.system(.callout, design: .rounded, weight: .semibold)` (13pt) |
| Metric label | `.footnote` (11pt) with secondary color |
| Metric value | `.system(.subheadline, design: .rounded, weight: .semibold).monospacedDigit()` (12pt) |
| Primary readout | `.system(.title3, design: .rounded, weight: .semibold).monospacedDigit()` (19pt) |
| Small metadata | `.caption.monospacedDigit()` (10pt) |
| Control pill text | `.system(.footnote, design: .rounded, weight: .semibold)` (11pt) |
| Inline chart labels | `.caption.monospacedDigit()` (10pt) |
| Fixed-size icons | 11 pt (small), 12 pt (trailing), 13 pt (leading), 14-15 pt (path/toolbar) |

Chinese UI labels should be short and scannable. Prefer `请落子`, `AI 思考`, `终局计算`, `局势接近` over longer explanatory strings inside compact controls.

## 4. Component Stylings

### Panels

Panels use `surface.panel`, 8 px corner radius, 1 px hairline border, and a soft stacked shadow. They should contain one coherent job: overview, captures, timeline, primary analysis, engine details, or diagnostics.

Do not nest cards inside cards. If a panel needs internal grouping, use dividers, compact rows, or a soft inline strip.

### Toolbar

Toolbar height is 54 px. Group controls by intent:

- Primary game flow: start/resume, pause, new game, **save**, **game library**.
- Board actions: undo, score, reset.
- Configuration: human color, model, level.
- Engine state: fixed-width status pill at the trailing edge.

Save (`square.and.arrow.down`) is disabled when no moves have been played. Game library (`folder`) opens a sheet listing all saved games for load/delete. Both sit between "new game" and the first divider.

Use icon buttons for compact actions and native menus/pickers for option sets. Destructive or disruptive actions should be visually de-emphasized or placed later in the group.

### Board Stage

The board stage is square, centered, and allowed to dominate the layout. It uses a subtle white backing plane plus deeper shadow than panels. A compact status strip may sit near the board to show turn, last move, and engine activity.

### Analysis Readouts

Winrate and score lead should be primary visual readouts, above engine implementation details. Use black/white bar metaphors for black/white ownership, and teal only for the interpreted position badge or active thinking signal.

### Replay / Review Mode

When the user enters review mode (by loading a saved game from the game library, or clicking a move in the timeline), the app switches to a read-only replay state:

- **MoveTimelineView** shows a Slider progress bar and step-forward/step-backward buttons between the panel header and the move list. The slider works with a local `@State` value to avoid flooding the engine during drag; it commits only on drag-end (`onEditingChanged`).
- Each move row is clickable, calling `jumpToMove(_:)` to navigate to that position.
- In review mode, the right-side value column shows **historical winrate** (from `moveAnalysisHistory`) instead of move elapsed time.
- The current step is highlighted with `AppTheme.tealSoft.opacity(0.72)`.
- **AnalysisPanel**: the header changes from "AI 分析" to "复盘分析" (icon from `brain.head.profile` to `clock.arrow.circlepath`). Winrate/lead/score data comes from the historical snapshot at the current replay position, not from the live engine.
- **Continue Game** button: sets `isReviewMode = false`, restores `phase = .playing`, and resumes the display timer. If the human color needs an AI opening move, `requestOpeningMove()` is triggered.
- **Exit Review**: replays all recorded moves to return to the final position and exits review mode.
- Navigation uses a **generation counter** (`jumpGeneration`) to cancel superseded async jump requests, preventing overlapping GTP commands that would corrupt board state.

### Game Timer

A real-time display timer tracks elapsed game time. A `Timer` fires every 1 second, incrementing `displayTick`. The `totalElapsedText` computed property depends on `displayTick`, causing only the time label to re-render.

- `gameStartTime` is set when the game begins or resumes; `accumulatedElapsed` stores time accrued before a pause.
- Per-move timestamps are stored as cumulative seconds in `moveElapsedAtMove: [TimeInterval]`, recorded via `totalElapsedSeconds` at each move.
- On pause: `accumulatedElapsed` is incremented, `gameStartTime` is cleared, timer stops.
- On resume: `gameStartTime` is set to `Date()`, timer starts.
- Both `accumulatedElapsed` and `moveElapsedAtMove` are persisted in `SavedGameSession` and `SavedGameFile`.

### Game Persistence

Games are saved as JSON files under `~/Documents/katagogo/games/`, managed by `GameStore` (a singleton in `Services/GameStore.swift`).

**File format** (`SavedGameFile`, each game one `.json` file):
- `version` (Int, currently 1)
- `gameInfo` (`GameInfo`): boardSize, komi, handicap, humanColor, engineModel, savedAt
- `finalScore` (String?, optional)
- `totalElapsedSeconds` (TimeInterval)
- `moves` ([SavedMoveWithAnalysis]): per-move color, vertex, winrateBlack, leadBlack, elapsedAtMove

**Index file** (`_index.json`): a lightweight array of `GameIndexEntry` (id, savedAt, humanColor, moveCount, finalScore, engineModel) for the game library list, sorted by savedAt descending.

**Saving**: `GameViewModel.saveGame()` builds a `SavedGameFile` from `recordedMoves` + `moveAnalysisHistory` + current state, then calls `GameStore.shared.save()`.

**Loading**: `GameViewModel.loadGame(id:)` reads the file, resets local state, starts the engine, replays all moves via GTP `play` commands, and restores `recordedMoves`, `moveElapsedAtMove`, and `moveAnalysisHistory`. The game then opens in review mode at the final position.

**Per-move analysis recording**: `MoveAnalysisSnapshot` (moveNumber, winrateBlack, leadBlack, evaluationAccuracy, currentPlayer) is appended to `moveAnalysisHistory` in `refreshUI()` whenever `moveCount` changes. The array is trimmed in `undo()` and cleared in `resetLocalBoardState()`.

**Game Library UI** (`GameLibraryView`): a modal sheet (460×440 pt) listing all saved games. Each row shows date, engine model, final score (if available), move count, and human color. Buttons: load (teal arrow) and delete (trash icon). Empty state: tray icon + "暂无保存的棋局" text.

### Engine Configuration

The default engine backend is **Metal** (compiled from KataGo source with `-DUSE_BACKEND=METAL`), which offloads neural net inference to the Apple Silicon GPU. This keeps CPU usage near idle during AI thinking, essential for system responsiveness on 8GB unified-memory machines.

**`gtp.cfg` key settings** (in `kata-engine/`):
- `numSearchThreads = 4` — balanced for 8GB unified memory
- `nnCacheSizePowerOfTwo = 20` — limits neural net cache to ~1 GB (2^20 × ~1.5 KB per entry)
- `maxTime = 1.0` — wall-clock ceiling per move for interactive play

**Weight files** are managed via `EngineSettingsView` file picker. The recommended default is `b18c384` (18-block, 384-channel) for daily play; `b6c64` (lightweight) for debugging and fast iteration.

## 5. Layout Principles

Use a three-zone desktop layout:

- Left rail: game overview, captures, move timeline.
- Center: board and board-proximate status.
- Right rail: AI analysis and engine details.

The center board owns the largest visual area. Side rails should stay narrow and information-dense. At compact widths, reduce spacing and labels before shrinking the board too far.

Use 12 px spacing for compact layouts and 18 px spacing for comfortable desktop layouts. Panel internals generally use 10-14 px spacing and 14 px padding.

## 6. Depth & Elevation

Use restrained native depth:

| Level | Use | Shadow |
|---|---|---|
| 0 | App canvas | none |
| 1 | Toolbar | hairline only |
| 2 | Panels | `0 6 10 rgba(0,0,0,0.04)` |
| 3 | Board stage | `0 10 15 rgba(0,0,0,0.18)` plus `0 1 2 rgba(0,0,0,0.05)` |
| 4 | Tooltips/menus | dark surface or native menu chrome |

Avoid generic heavy shadows. The board can have the strongest depth because it is the physical object in the workspace.

## 7. Do's and Don'ts

Do:

- Keep the board visually dominant.
- Use native controls where they improve macOS familiarity.
- Keep metrics aligned and easy to scan.
- Use monospaced digits for winrate, score lead, counts, and time.
- Show engine activity near the board as well as in the toolbar.
- Record per-move analysis snapshots for replay: winrate, lead, evaluation accuracy.
- Serialize games as JSON in `~/Documents/katagogo/games/` with a lightweight index.
- Use generation counters for async navigation to cancel stale GTP requests.
- Debounce slider-driven navigation: commit on drag-end, not on every value change.

Don't:

- Turn teal into a page-wide palette.
- Add decorative gradients or marketing hero treatments.
- Make every panel equally important.
- Hide critical engine errors only in alerts.
- Use long explanatory copy inside compact UI controls.
- Use binary locks for async GTP replay navigation — use generation-based cancellation.
- Call `jumpToMove` on every slider value change — use local `@State` + `onEditingChanged`.

## 8. Responsive Behavior

The app minimum window is 940 x 620. Below 1080 px width, toolbar labels collapse where possible and side rails tighten. The board remains square and centered. Side rail widths should stay readable: left rail around 174-224 px, right rail around 220-284 px.

Text must not overflow controls. Prefer shorter labels, fixed-width status surfaces, truncation in model names, and monospaced numeric rows.

## 9. Agent Prompt Guide

When editing KataGoGo UI:

- Follow this file and `KataGoGo/DesignSystem.swift`.
- Preserve the quiet macOS training-tool feeling.
- Add tokens before scattering new magic numbers.
- Keep the board central and reduce panel competition.
- Use teal only for AI/active signals.
- Prefer icon buttons, menus, segmented controls, and compact metric rows over verbose text buttons.
- Verify SwiftUI changes with an Xcode build when possible.

When working with engine interaction:

- Async GTP commands must be protected against concurrency — use generation counters, not binary locks.
- Slider controls that trigger GTP replay must debounce with local `@State` + `onEditingChanged`.
- Store per-move analysis snapshots during play; do not rely on re-running the engine for replay data.
- Game persistence uses JSON files under `~/Documents/katagogo/games/` with a `_index.json` manifest.
- The left-rail panel is ~174–224 px wide — review controls must fit within this constraint.
- `GameStore` is the single entry point for save/load/delete/list operations.

When working with fonts:

- Minimum readable size is `.caption` (10pt). Never use `.caption2` (9pt).
- Metric labels use `.footnote` (11pt). Panel titles use `.callout` (13pt).
- Fixed icon sizes: 11 pt (small), 12 pt (trailing), 13 pt (leading), 14-15 pt (path/toolbar).
