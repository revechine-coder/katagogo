# KataGoGo — macOS Go Game Client Design

> **Project:** Cross-platform Go game client with KataGo AI engine
> **Phase 1:** macOS app (SwiftUI + Rust shared core)
> **Date:** 2026-05-16

## Overview

A native macOS Go (围棋/weiqui) game application powered by KataGo GTP engine. Users play against AI with adjustable difficulty levels, real-time winrate/lead display, full-board mini-map with move number annotations, undo support, and SGF export.

## Tech Stack

- **Shared core:** Rust (compiled to `staticlib` for static linking)
- **UI layer:** SwiftUI (native macOS with Metal rendering for board)
- **Engine:** KataGo v1.15+ GTP subprocess
- **FFI:** C ABI from Rust, called via Swift `@_silgen_name` + bridging header

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    macOS SwiftUI Layer                    │
│  ┌─────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │MiniMap  │  │BoardView     │  │AnalysisPanel       │  │
│  │(CG)     │  │(Metal/CG)    │  │- WinrateBar        │  │
│  │+编号     │  │+手势缩放      │  │- ScoreLeadView     │  │
│  └─────────┘  └──────────────┘  └────────────────────┘  │
│                     │                                    │
│             GameViewModel (ObservableObject)              │
│                     │                                    │
│               GoCoreBridge (Swift FFI)                   │
└──────────────────────┬───────────────────────────────────┘
                       │ C ABI
┌──────────────────────▼───────────────────────────────────┐
│              Rust go_core staticlib                        │
│  GameState | MoveHistory | GtpClient | AnalysisData       │
│  SgfParser | BoardRenderer (→ RenderFrame)               │
└──────────────────────┬───────────────────────────────────┘
                       │ stdin/stdout
              ┌────────▼────────┐
              │  KataGo GTP     │
              │  (subprocess)   │
              └─────────────────┘
```

## Data Flow (Play → AI Respond)

```
1. User taps intersection on BoardView
2. BoardGesture → BoardViewModel.tap(at: (col, row))
3. GameViewModel.play(at:) — fires Task.detached:
   a. GoCoreBridge.play(color, vertex) — Rust: GameState.record + GtpClient.send("play ...")
   b. GoCoreBridge.genmove(opponent) — Rust: GtpClient.send("genmove ...") → parse stderr
   c. GoCoreBridge.getRenderFrame() → RenderFrame struct
   d. Back to MainActor: update @Published vars
4. SwiftUI reacts → BoardView/MiniMapView/AnalysisPanel redraw
```

## Window Layout

```
┌─────────────────────────────────────────────────────────────┐
│  [Pass] [Undo] [Score]        AI Level: [1d-3d ▼]         │
├──────────┬────────────────────────────┬────────────────────┤
│          │                            │                    │
│  MiniMap  │       Main Board          │  Winrate Bar       │
│  全编号    │    (Metal/CG Render)      │  67.3% ████░░░     │
│  步数显示  │                            │                    │
│          │                            │  目差形势           │
│   ↕可缩放 │                            │  黑+2.5目          │
│          │                            │  ████████░░        │
│          │                            │                    │
│          │                            │  当前: 黑棋 第27手   │
├──────────┴────────────────────────────┴────────────────────┤
│  状态 | 连接指示灯 | 引擎信息                              │
└─────────────────────────────────────────────────────────────┘
```

## Key Design Decisions

1. **Pure sync GTP via C ABI + Swift Task.detached for async** — simpler than running tokio in async C callback
2. **RenderFrame pattern** — Rust exports a flat data struct; SwiftUI just paints it, no game logic in UI
3. **FFI error handling** — C ABI returns `int` (0=ok, -1=error), last error string stored in Rust static
4. **KataGo process** — one process per app instance (simple), managed lifecycle in GoCoreBridge
5. **No frontend board rules** — all rule validation delegated to KataGo; GameState only tracks what KataGo confirmed

## Phased Delivery (macOS only)

### Phase 1A: Rust Core (Week 1)
- GameState + MoveHistory + RenderFrame + C ABI exports
- Testable via `cargo test` (no KataGo dependency for most tests)

### Phase 1B: macOS Skeleton (Week 2)
- Xcode project + Rust build integration
- GoCoreBridge Swift FFI layer
- GameViewModel + BoardViewModel
- CoreGraphics board rendering (no Metal yet)
- Readable play → AI respond loop
- MiniMapView with move numbers
- Undo/Redo via MoveHistory
- WinrateBar + ScoreLeadView

### Phase 1C: UI Polish (Week 3)
- Metal board rendering (if CG performance insufficient)
- Dark mode support
- Keyboard shortcuts (⌘Z undo, ⌘P pass, ⌘N new game, ⌘S score)
- SGF export
- Final score modal
- Preferences window (KataGo paths, default level)
- Auto-reconnect on engine crash
