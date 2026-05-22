# KataGoGo — Cross-Platform Go Game Client with KataGo AI

[English](#english) | [中文](#chinese)

---

<a name="chinese"></a>

## 中文介绍

**KataGoGo** 是一个基于 **KataGo** 强大 GTP 引擎驱动的跨平台围棋（Weiqi）客户端项目。

该项目的第一阶段专为 **macOS** 开发，采用 **SwiftUI** 构建原生精美界面，并使用 **Rust** 编写跨平台的高性能共享核心（`go_core`），两者通过零开销的 C ABI 进行静态链接交互。

### 核心功能

- **AI 对弈**：基于 KataGo GTP 引擎，支持多档难度（1d 到 3d 等）动态对局。
- **实时形势分析**：实时计算并渲染胜率条（Winrate Bar）与目差（Score Lead）。
- **全路复盘缩略图**：左侧提供带有全手顺编号的高性能迷你棋盘。
- **完美的悔棋/跳转系统**：支持无限次 Undo/Redo 与历史节点重新分支落子。
- **导出与兼容性**：支持标准 SGF 格式导出。

### 技术架构

项目使用清晰的**数据单向流**与**双层架构**：

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
                       │ C ABI (GoCoreBridge.swift)
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

1. **Rust 共享核心 (`go_core`)**：负责管理棋盘逻辑（GameState）、历史纪录（MoveHistory）、拉起并解析 KataGo 进程（GtpClient）的胜率及目差数据，最后将其包装为只读的渲染帧（RenderFrame）供 Swift 渲染，避免前端和核心层数据不一致。
2. **Swift 桥接层 (`GoCoreBridge.swift`)**：利用 `@_silgen_name` 高速加载 Rust 导出的 C ABI，无需任何 Obj-C 繁琐中介。
3. **SwiftUI 展现层**：纯声明式 UI，通过 Canvas/Metal 进行高效渲染绘制，确保超低开销。

### 源码结构

```text
.
├── Cargo.toml                  # Rust 依赖与 staticlib 配置
├── Makefile                    # 快捷一键编译、测试、链接脚本
├── GoCoreBridge.swift          # Swift 对 FFI 的面向对象闭合封装
├── docs/                       # 设计规格与阶段性实现方案目录
│   └── superpowers/
│       ├── specs/              # macOS 精细设计交互规格文档
│       └── plans/              # 阶段 1A (Rust) / 1B (Swift) 开发执行计划
└── src/                        # Rust 核心源代码
    ├── lib.rs                  # FFI ABI 核心暴露定义
    ├── ffi.rs                  # 专为 C ABI 设计的类型安全导出和数据转化
    ├── game_state.rs           # 棋盘规则和执子状态校验
    ├── gtp_client.rs           # 异步/同步高并发 KataGo 子进程控制和解析
    ├── render_frame.rs         # 紧凑型扁平化棋盘结构，直接映射至 Swift
    └── tests/                  # 16+ 个覆盖完整的 Rust 单元测试
```

### 快速编译与测试

在具备 Rust 环境的 macOS/Linux 主机上运行：

**1. 运行所有单元测试**
```bash
make test
```

**2. 编译 Release 静态链接库 (`libgo_core.a`)**
```bash
make release
```

---

<a name="english"></a>

## English Introduction

**KataGoGo** is a cross-platform Go (Weiqi) game application powered by the state-of-the-art **KataGo** GTP engine.

Phase 1 targets **macOS** with a native SwiftUI frontend statically linked to a high-performance **Rust** shared core (`go_core`) via a zero-overhead C ABI.

### Key Features

- **AI Play & Matchmaking**: Choose from multiple AI difficulty levels (e.g., 1d-3d) powered by KataGo.
- **Real-Time Analysis**: Dynamic winrate progress bars and real-time score leads (Points margin).
- **Move Annotated MiniMap**: A high-efficiency minimap on the sidebar showing the entire board annotated with full move sequences.
- **Robust History Branching**: Supports unlimited undo/redo with safe board-state branching.
- **SGF Exporting**: Generate standard SGF output for easy game reviewing.

### System Architecture

The project features a **unidirectional data flow** with decoupling between UI and core engine:

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
                       │ C ABI (GoCoreBridge.swift)
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

1. **Rust Shared Core (`go_core`)**: Tracks game states, move histories, and manages the lifecycle of the KataGo subprocess. Stderr is parsed in background threads. It outputs a data-only `RenderFrame` snapshot.
2. **Swift FFI Bridge (`GoCoreBridge.swift`)**: Uses Swift `@_silgen_name` to call Rust ABI directly, bypassing Objective-C wrappers entirely.
3. **SwiftUI Layer**: Renders the `RenderFrame` declaratively using CoreGraphics/Metal, ensuring maximum efficiency and responsiveness.

### Directory Layout

```text
.
├── Cargo.toml                  # Rust library manifest & crate-type settings
├── Makefile                    # Automation build and testing script
├── GoCoreBridge.swift          # Swift object-oriented wrapper around the C FFI
├── docs/                       # Project specifications and dev roadmaps
│   └── superpowers/
│       ├── specs/              # Detailed macOS specifications & UI designs
│       └── plans/              # Action plans for Phase 1A (Rust) & 1B (Swift)
└── src/                        # Rust Core Source Files
    ├── lib.rs                  # Library entrypoint & FFI boundary
    ├── ffi.rs                  # Safe C ABI data conversions
    ├── game_state.rs           # Core Go logic & move validations
    ├── gtp_client.rs           # Non-blocking GTP subprocess controller
    ├── render_frame.rs         # Flat UI data transfer structure
    └── tests/                  # Robust suite of unit tests (16+ tests)
```

### Build & Run Tests

Ensure you have Rust toolchain installed, then run inside the `go-core` directory:

**1. Run Unit Tests**
```bash
make test
```

**2. Compile Release Static Library (`libgo_core.a`)**
```bash
make release
```
