# KataGoGo macOS — Phase 1B: SwiftUI App Skeleton

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan.

**Goal:** A working macOS app that shows a Go board, lets the user click to play, and sees KataGo respond with AI move + winrate/lead analysis.

**Architecture:** SwiftUI app with `GoCoreBridge` calling into Rust `go_core` static lib. CoreGraphics board rendering (Metal later if needed). `GameViewModel` as single `ObservableObject` driving all UI state.

**Tech Stack:** SwiftUI, CoreGraphics, Rust `go_core` static library (C ABI)

---

## File Structure

```
KataGoGo.xcodeproj/
KataGoGo/
├── KataGoGoApp.swift         # @main entry
├── AppState.swift            # Global state (persistent settings)
├── Views/
│   ├── ContentView.swift     # Main window: 3-column layout
│   ├── Board/
│   │   ├── BoardView.swift       # SwiftUI wrapper + gesture handler
│   │   └── BoardRenderer.swift   # CoreGraphics drawing
│   ├── MiniMap/
│   │   └── MiniMapView.swift     # Full-board thumbnail with move numbers
│   ├── Sidebar/
│   │   ├── AnalysisPanel.swift   # Right sidebar container
│   │   ├── WinrateBar.swift      # Winrate bar + percentage
│   │   └── ScoreLeadView.swift   # Score lead display
│   └── Toolbar/
│       ├── GameToolbar.swift     # Pass/Undo/Score/Level buttons
│       └── LevelPicker.swift     # AI strength selector
├── ViewModels/
│   └── GameViewModel.swift   # @Observable game state
├── Services/
│   └── GoCoreBridge.swift    # Already written (FFI bridge)
├── SharedCore/
│   ├── go_core.h             # C header (copy from Rust project)
│   └── libgo_core.a          # Built by Rust (build script)
└── Assets.xcassets/
```

---

### Task 1: Create Xcode project

**Files:**
- Create: Xcode project (use Xcode UI, then add files manually)

**Manual steps on your Mac:**

```bash
# 1. Open Xcode → New Project → macOS → App → SwiftUI
#    Product Name: "KataGoGo"
#    Team: (none needed)
#    Organization Identifier: com.yourname
#    Interface: SwiftUI
#    Language: Swift
#    Save to: ~/Desktop/KataGoGo/

# 2. Copy Rust files into project
mkdir -p ~/Desktop/KataGoGo/KataGoGo/SharedCore
cp ~/Downloads/go-core/src/go_core.h ~/Desktop/KataGoGo/KataGoGo/SharedCore/
cp ~/Downloads/go-core/GoCoreBridge.swift ~/Desktop/KataGoGo/KataGoGo/Services/

# 3. Build Rust lib (macOS ARM64)
cd ~/Downloads/go-core
cargo build --release
cp target/release/libgo_core.a ~/Desktop/KataGoGo/KataGoGo/SharedCore/
```

**Xcode project settings:**
- Go to **Build Phases** → Link Binary With Libraries → Add `libgo_core.a`
- Go to **Build Settings** → Library Search Paths → add `$(PROJECT_DIR)/KataGoGo/SharedCore`
- Go to **Build Phases** → Add a new **Run Script Phase** (after Sources) to rebuild Rust automatically:

```bash
cd $PROJECT_DIR/../go-core
cargo build --release
cp target/release/libgo_core.a $PROJECT_DIR/KataGoGo/SharedCore/
```

- [ ] **Step 1:** Create Xcode project (manual)
- [ ] **Step 2:** Set up Rust build + Xcode integration (manual)
- [ ] **Step 3:** Add files in Xcode → right-click group → Add Files → select all .swift files
- [ ] **Step 4:** Build to verify (should compile, app launches blank window)

---

### Task 2: AppState + GameViewModel

**Files:**
- Create: `KataGoGo/AppState.swift`
- Create: `KataGoGo/ViewModels/GameViewModel.swift`

- [ ] **Step 1: Write AppState.swift**

```swift
import Foundation

struct AppSettings {
    var kataGoBinaryPath: String = "/opt/katago/katago"
    var kataGoConfigPath: String = "/opt/katago/gtp.cfg"
    var kataGoModelPath: String = "/opt/katago/models/kata1-15b.bin.gz"
    var boardSize: Int = 19
    var aiLevel: Int = 4 // 1d-3d default
}

@Observable
final class AppState {
    var settings = AppSettings()
    
    static let shared = AppState()
}
```

- [ ] **Step 2: Write GameViewModel.swift**

```swift
import SwiftUI
import Observation

enum GamePhase {
    case idle       // engine not started
    case connecting // starting KataGo
    case playing    // normal play
    case waiting    // waiting for AI move
    case finished   // game ended
}

@Observable
final class GameViewModel {
    
    // ── Public state ───────────────────────────────────
    
    var phase: GamePhase = .idle
    var currentPlayer: String = "b"
    var moveCount: Int = 0
    var winrateBlack: Double = 0.5
    var lead: Double = 0.0
    var capturesBlack: Int = 0
    var capturesWhite: Int = 0
    var lastMove: (col: Int, row: Int)? = nil
    var errorMessage: String? = nil
    
    // A 19x19 array of optional stone colors: nil = empty, true = black, false = white
    var board: [[Bool?]] = Array(repeating: Array(repeating: nil, count: 19), count: 19)
    
    // Move labels for mini-map: (col, row, moveNumber)
    var moveLabels: [(col: Int, row: Int, moveNumber: Int)] = []
    
    // ── Engine ─────────────────────────────────────────
    
    private let engine = GoCoreBridge()
    
    // ── Lifecycle ──────────────────────────────────────
    
    func startGame() {
        phase = .connecting
        
        guard engine.create() else {
            errorMessage = "Failed to create engine"
            phase = .idle
            return
        }
        
        let appState = AppState.shared
        do {
            try engine.start(
                binaryPath: appState.settings.kataGoBinaryPath,
                configPath: appState.settings.kataGoConfigPath,
                modelPath: appState.settings.kataGoModelPath,
                boardSize: appState.settings.boardSize,
                timeout: 120.0
            )
            engine.setLevel(appState.settings.aiLevel)
            phase = .playing
            refreshUI()
        } catch {
            errorMessage = error.localizedDescription
            phase = .idle
        }
    }
    
    func endGame() {
        engine.close()
        engine.destroy()
        phase = .idle
    }
    
    // ── Actions ────────────────────────────────────────
    
    func play(at col: Int, row: Int) {
        guard phase == .playing else { return }
        guard col >= 0, col < 19, row >= 0, row < 19 else { return }
        guard board[row][col] == nil else { return }
        
        let vertex = vertexFromPoint(col: col, row: row)
        print("Player plays \(currentPlayer) at \(vertex)")
        
        phase = .waiting
        
        // Run in background to not block UI
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            
            do {
                // 1. Play player's move
                try self.engine.play(color: self.currentPlayer, vertex: vertex)
                
                // 2. AI response
                let aiColor = self.currentPlayer == "b" ? "w" : "b"
                let result = try self.engine.genmove(color: aiColor)
                print("AI played \(aiColor) at \(result.vertex), winrate=\(result.winrate), lead=\(result.lead)")
                
                DispatchQueue.main.async {
                    self.refreshUI(from: result)
                    self.phase = .playing
                }
            } catch {
                DispatchQueue.main.async {
                    self.phase = .playing
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }
    
    func undo() {
        guard phase == .playing else { return }
        _ = engine.undo() // undoes AI + player move pair
        refreshUI()
    }
    
    func pass() {
        guard phase == .playing else { return }
        phase = .waiting
        
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            do {
                try self.engine.play(color: self.currentPlayer, vertex: "pass")
                let aiColor = self.currentPlayer == "b" ? "w" : "b"
                let result = try self.engine.genmove(color: aiColor)
                
                DispatchQueue.main.async {
                    self.refreshUI(from: result)
                    self.phase = .playing
                }
            } catch {
                DispatchQueue.main.async {
                    self.phase = .playing
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }
    
    func finalScore() -> String? {
        guard phase == .playing else { return nil }
        // We don't have final_score in the FFI yet, but the engine supports it
        return nil
    }
    
    func setLevel(_ level: Int) {
        engine.setLevel(level)
        AppState.shared.settings.aiLevel = level
    }
    
    func resetGame() {
        do {
            try engine.reset()
            board = Array(repeating: Array(repeating: nil, count: 19), count: 19)
            moveLabels = []
            moveCount = 0
            currentPlayer = "b"
            winrateBlack = 0.5
            lead = 0.0
            lastMove = nil
            phase = .playing
        } catch {
            errorMessage = error.localizedDescription
        }
    }
    
    // ── UI refresh ─────────────────────────────────────
    
    private func refreshUI(from genmoveResult: (vertex: String, winrate: Double, lead: Double)? = nil) {
        // Get render frame from engine
        if let frame = engine.getRenderFrame() {
            self.board = Self.stonesToBoard(frame.stones)
            self.moveLabels = frame.moveLabels
            self.moveCount = frame.moveCount
            self.currentPlayer = frame.currentPlayer
            self.lastMove = frame.lastMove
        }
        
        if let analysis = engine.getAnalysis() {
            self.winrateBlack = analysis.winrateBlack
            self.lead = analysis.lead
            self.capturesBlack = analysis.capturesBlack
            self.capturesWhite = analysis.capturesWhite
        }
    }
    
    private static func stonesToBoard(_ stones: [(col: Int, row: Int, isBlack: Bool)]) -> [[Bool?]] {
        var board = Array(repeating: Array(repeating: nil as Bool?, count: 19), count: 19)
        for stone in stones {
            guard stone.col >= 0, stone.col < 19, stone.row >= 0, stone.row < 19 else { continue }
            board[stone.row][stone.col] = stone.isBlack
        }
        return board
    }
    
    // ── Helpers ────────────────────────────────────────
    
    private let columns = "ABCDEFGHJKLMNOPQRST"
    
    func vertexFromPoint(col: Int, row: Int) -> String {
        let colChar = columns[columns.index(columns.startIndex, offsetBy: col)]
        let rowNum = 19 - row
        return "\(colChar)\(rowNum)"
    }
}
```

- [ ] **Step 3: Build check**

Build project in Xcode to make sure it compiles (⌘B). Expect maybe linker errors if libgo_core.a isn't placed yet — that's OK for now.

---

### Task 3: BoardView + BoardRenderer

**Files:**
- Create: `KataGoGo/Views/Board/BoardRenderer.swift`
- Create: `KataGoGo/Views/Board/BoardView.swift`

- [ ] **Step 1: Write BoardRenderer.swift**

```swift
import SwiftUI

struct BoardRenderer {
    
    let boardSize: Int
    
    /// Draws the Go board onto a CGContext.
    /// - Parameters:
    ///   - context: CGContext to draw on
    ///   - size: Size of the drawing area in points
    ///   - stones: Array of (col, row, isBlack)
    ///   - lastMove: Optional (col, row) of the most recent move
    ///   - moveLabels: Move number labels for mini-map mode
    ///   - showCoordinates: Whether to show edge coordinates
    ///   - miniMap: If true, render small version with move numbers
    func drawBoard(context: CGContext, size: CGSize,
                   stones: [(col: Int, row: Int, isBlack: Bool)],
                   lastMove: (col: Int, row: Int)?,
                   moveLabels: [(col: Int, row: Int, moveNumber: Int)] = [],
                   showCoordinates: Bool = true,
                   miniMap: Bool = false) {
        
        let padding: CGFloat = miniMap ? 4 : size.width * 0.06
        let gridSize = (size.width - 2 * padding) / CGFloat(boardSize - 1)
        let stoneRadius = gridSize * 0.44
        
        // ── Background (wood texture) ──────────────────
        if miniMap {
            context.setFillColor(CGColor(red: 0.84, green: 0.64, blue: 0.40, alpha: 1.0))
        } else {
            // Simple wood color
            context.setFillColor(CGColor(red: 0.84, green: 0.64, blue: 0.40, alpha: 1.0))
        }
        context.fill(CGRect(origin: .zero, size: size))
        
        // ── Grid lines ─────────────────────────────────
        context.setStrokeColor(CGColor(red: 0.18, green: 0.14, blue: 0.10, alpha: 1.0))
        context.setLineWidth(miniMap ? 0.5 : 1.0)
        
        for i in 0..<boardSize {
            let x = padding + CGFloat(i) * gridSize
            let yTop = padding
            let yBottom = padding + CGFloat(boardSize - 1) * gridSize
            context.move(to: CGPoint(x: x, y: yTop))
            context.addLine(to: CGPoint(x: x, y: yBottom))
            
            let y = padding + CGFloat(i) * gridSize
            let xLeft = padding
            let xRight = padding + CGFloat(boardSize - 1) * gridSize
            context.move(to: CGPoint(x: xLeft, y: y))
            context.addLine(to: CGPoint(x: xRight, y: y))
        }
        context.strokePath()
        
        // ── Star points ────────────────────────────────
        let starPoints = [(3,3), (3,9), (3,15), (9,3), (9,9), (9,15), (15,3), (15,9), (15,15)]
        let starRadius = miniMap ? 1.5 : gridSize * 0.08
        context.setFillColor(CGColor(red: 0.18, green: 0.14, blue: 0.10, alpha: 1.0))
        for (col, row) in starPoints {
            guard col < boardSize, row < boardSize else { continue }
            let cx = padding + CGFloat(col) * gridSize
            let cy = padding + CGFloat(row) * gridSize
            context.fillEllipse(in: CGRect(x: cx - starRadius, y: cy - starRadius,
                                           width: starRadius * 2, height: starRadius * 2))
        }
        
        // ── Stones ─────────────────────────────────────
        for (col, row, isBlack) in stones {
            let cx = padding + CGFloat(col) * gridSize
            let cy = padding + CGFloat(row) * gridSize
            drawStone(context: context, center: CGPoint(x: cx, y: cy),
                      radius: stoneRadius, isBlack: isBlack, miniMap: miniMap)
        }
        
        // ── Last move marker ───────────────────────────
        if let (col, row) = lastMove, !miniMap {
            let cx = padding + CGFloat(col) * gridSize
            let cy = padding + CGFloat(row) * gridSize
            context.setStrokeColor(CGColor(red: 0.91, green: 0.30, blue: 0.24, alpha: 0.8))
            context.setLineWidth(2.5)
            context.strokeEllipse(in: CGRect(x: cx - stoneRadius * 0.6, y: cy - stoneRadius * 0.6,
                                             width: stoneRadius * 1.2, height: stoneRadius * 1.2))
        }
        
        // ── Move numbers (mini-map) ───────────────────
        if miniMap {
            let fontSize = max(6, gridSize * 0.35)
            let font = CTFontCreateWithName("Helvetica" as CFString, fontSize, nil)
            
            for label in moveLabels {
                let cx = padding + CGFloat(label.col) * gridSize
                let cy = padding + CGFloat(label.row) * gridSize
                
                let text = "\(label.moveNumber)" as CFString
                let attributes: [CFString: Any] = [
                    kCTFontAttributeName: font,
                    kCTForegroundColorAttributeName: CGColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 0.7),
                ]
                let attrStr = CFAttributedStringCreate(nil, text, attributes as CFDictionary)
                let line = CTLineCreateWithAttributedString(attrStr!)
                let bounds = CTLineGetBoundsWithOptions(line, [])
                
                context.textPosition = CGPoint(x: cx - bounds.width / 2, y: cy - bounds.height / 2)
                CTLineDraw(line, context)
            }
        }
        
        // ── Coordinates ────────────────────────────────
        if showCoordinates && !miniMap {
            let fontSize = max(8, gridSize * 0.3)
            let font = CTFontCreateWithName("Helvetica" as CFString, fontSize, nil)
            
            let columns = "ABCDEFGHJKLMNOPQRST"
            for i in 0..<boardSize {
                let x = padding + CGFloat(i) * gridSize
                
                // Bottom: column letters
                let colChar = String(columns[columns.index(columns.startIndex, offsetBy: i)])
                let text = colChar as CFString
                let attr: [CFString: Any] = [
                    kCTFontAttributeName: font,
                    kCTForegroundColorAttributeName: CGColor(red: 0.3, green: 0.25, blue: 0.2, alpha: 0.8),
                ]
                let attrStr = CFAttributedStringCreate(nil, text, attr as CFDictionary)
                let line = CTLineCreateWithAttributedString(attrStr!)
                let bounds = CTLineGetBoundsWithOptions(line, [])
                context.textPosition = CGPoint(
                    x: x - bounds.width / 2,
                    y: size.height - padding + (gridSize * 0.15)
                )
                CTLineDraw(line, context)
                
                // Left: row numbers
                let rowNum = boardSize - i
                let rowText = "\(rowNum)" as CFString
                let rowAttrStr = CFAttributedStringCreate(nil, rowText, attr as CFDictionary)
                let rowLine = CTLineCreateWithAttributedString(rowAttrStr!)
                let rowBounds = CTLineGetBoundsWithOptions(rowLine, [])
                context.textPosition = CGPoint(
                    x: padding - gridSize * 0.15 - rowBounds.width,
                    y: padding + CGFloat(i) * gridSize - rowBounds.height / 2
                )
                CTLineDraw(rowLine, context)
            }
        }
    }
    
    private func drawStone(context: CGContext, center: CGPoint, radius: CGFloat,
                           isBlack: Bool, miniMap: Bool) {
        if miniMap {
            // Simple filled circle for mini-map
            context.setFillColor(isBlack ? CGColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 1.0)
                                          : CGColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0))
            context.fillEllipse(in: CGRect(x: center.x - radius, y: center.y - radius,
                                           width: radius * 2, height: radius * 2))
            return
        }
        
        // Gradient stone rendering
        let colors: [CGColor]
        if isBlack {
            colors = [
                CGColor(red: 0.4, green: 0.4, blue: 0.4, alpha: 1.0),  // highlight
                CGColor(red: 0.05, green: 0.05, blue: 0.05, alpha: 1.0), // dark
            ]
        } else {
            colors = [
                CGColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0),
                CGColor(red: 0.85, green: 0.85, blue: 0.85, alpha: 1.0),
            ]
        }
        
        let gradient = CGGradient(
            colorsSpace: CGColorSpaceCreateDeviceRGB(),
            colors: colors as CFArray,
            locations: [0.0, 1.0]
        )
        
        let stoneRect = CGRect(x: center.x - radius, y: center.y - radius,
                               width: radius * 2, height: radius * 2)
        context.saveGState()
        context.addEllipse(in: stoneRect)
        context.clip()
        
        context.drawRadialGradient(
            gradient!,
            startCenter: CGPoint(x: center.x - radius * 0.25, y: center.y - radius * 0.3),
            startRadius: 0,
            endCenter: center,
            endRadius: radius,
            options: []
        )
        context.restoreGState()
        
        // Border
        context.setStrokeColor(isBlack ? CGColor(red: 0, green: 0, blue: 0, alpha: 1)
                                        : CGColor(red: 0.7, green: 0.7, blue: 0.7, alpha: 1))
        context.setLineWidth(0.5)
        context.strokeEllipse(in: stoneRect)
    }
}
```

- [ ] **Step 2: Write BoardView.swift**

```swift
import SwiftUI

struct BoardView: View {
    
    let viewModel: GameViewModel
    let renderer = BoardRenderer(boardSize: 19)
    
    var body: some View {
        GeometryReader { geometry in
            Canvas { contextRef, size in
                let ctx = contextRef
                // Draw board into the current CGContext
                let renderSize = size
                let rect = CGRect(origin: .zero, size: renderSize)
                
                // We need to use a CGContext directly; Canvas in SwiftUI 3+
                // uses a GraphicsContext. For clarity, we'll use a custom NSViewRepresentable.
                // This is a placeholder — see next step.
            }
        }
    }
}

// ── NSViewRepresentable for CoreGraphics rendering ─────────

struct BoardCanvas: NSViewRepresentable {
    
    let viewModel: GameViewModel
    
    func makeNSView(context: Context) -> BoardNSView {
        let view = BoardNSView()
        view.viewModel = viewModel
        return view
    }
    
    func updateNSView(_ nsView: BoardNSView, context: Context) {
        nsView.viewModel = viewModel
        nsView.needsDisplay = true
    }
}

class BoardNSView: NSView {
    
    weak var viewModel: GameViewModel?
    private let renderer = BoardRenderer(boardSize: 19)
    
    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext,
              let vm = viewModel else { return }
        
        var stones: [(col: Int, row: Int, isBlack: Bool)] = []
        for row in 0..<19 {
            for col in 0..<19 {
                if let isBlack = vm.board[row][col] {
                    stones.append((col, row, isBlack))
                }
            }
        }
        
        renderer.drawBoard(
            context: ctx,
            size: bounds.size,
            stones: stones,
            lastMove: vm.lastMove,
            showCoordinates: true,
            miniMap: false
        )
    }
    
    override func mouseDown(with event: NSEvent) {
        guard let vm = viewModel else { return }
        
        let point = convert(event.locationInWindow, from: nil)
        let padding = bounds.width * 0.06
        let gridSize = (bounds.width - 2 * padding) / 18.0
        
        let col = Int(round((point.x - padding) / gridSize))
        let row = Int(round((point.y - padding) / gridSize))
        
        guard col >= 0, col < 19, row >= 0, row < 19 else { return }
        vm.play(at: col, row: row)
    }
    
    override var acceptsFirstResponder: Bool { true }
}
```

- [ ] **Step 3: Build check**

Build project. Should compile (may have one or two Swift warnings, no errors).

---

### Task 4: MiniMapView

**Files:**
- Create: `KataGoGo/Views/MiniMap/MiniMapView.swift`

- [ ] **Step 1: Write MiniMapView.swift**

```swift
import SwiftUI

struct MiniMapView: View {
    let viewModel: GameViewModel
    let renderer = BoardRenderer(boardSize: 19)
    
    var body: some View {
        VStack(spacing: 2) {
            Text("棋局总览")
                .font(.caption)
                .foregroundColor(.secondary)
            
            MiniMapCanvas(viewModel: viewModel)
                .frame(width: 160, height: 160)
                .cornerRadius(6)
                .shadow(radius: 2)
        }
    }
}

struct MiniMapCanvas: NSViewRepresentable {
    
    let viewModel: GameViewModel
    
    func makeNSView(context: Context) -> MiniMapNSView {
        let view = MiniMapNSView()
        view.viewModel = viewModel
        return view
    }
    
    func updateNSView(_ nsView: MiniMapNSView, context: Context) {
        nsView.viewModel = viewModel
        nsView.needsDisplay = true
    }
}

class MiniMapNSView: NSView {
    
    weak var viewModel: GameViewModel?
    private let renderer = BoardRenderer(boardSize: 19)
    
    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext,
              let vm = viewModel else { return }
        
        var stones: [(col: Int, row: Int, isBlack: Bool)] = []
        for row in 0..<19 {
            for col in 0..<19 {
                if let isBlack = vm.board[row][col] {
                    stones.append((col, row, isBlack))
                }
            }
        }
        
        renderer.drawBoard(
            context: ctx,
            size: bounds.size,
            stones: stones,
            lastMove: nil, // no last-move marker in mini-map
            moveLabels: vm.moveLabels,
            showCoordinates: false,
            miniMap: true
        )
    }
    
    override func mouseDown(with event: NSEvent) {
        guard let vm = viewModel else { return }
        let point = convert(event.locationInWindow, from: nil)
        let padding: CGFloat = 4
        let gridSize = (bounds.width - 2 * padding) / 18.0
        
        let col = Int(round((point.x - padding) / gridSize))
        let row = Int(round((point.y - padding) / gridSize))
        
        guard col >= 0, col < 19, row >= 0, row < 19 else { return }
        
        // Find the move number at this position
        if let label = vm.moveLabels.first(where: { $0.col == col && $0.row == row }) {
            print("Jump to move #\(label.moveNumber)")
        }
    }
}
```

- [ ] **Step 2: Build check**

---

### Task 5: AnalysisPanel (WinrateBar + ScoreLeadView)

**Files:**
- Create: `KataGoGo/Views/Sidebar/WinrateBar.swift`
- Create: `KataGoGo/Views/Sidebar/ScoreLeadView.swift`
- Create: `KataGoGo/Views/Sidebar/AnalysisPanel.swift`

- [ ] **Step 1: Write WinrateBar.swift**

```swift
import SwiftUI

struct WinrateBar: View {
    let winrateBlack: Double  // 0.0 - 1.0
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("胜率")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(String(format: "%.1f%%", winrateBlack * 100))
                    .font(.title3)
                    .fontWeight(.semibold)
                    .monospacedDigit()
            }
            
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    // Background bar
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color.gray.opacity(0.2))
                        .frame(height: 20)
                    
                    // Winrate fill (black's perspective)
                    RoundedRectangle(cornerRadius: 4)
                        .fill(LinearGradient(
                            colors: [.black, .gray],
                            startPoint: .leading,
                            endPoint: .trailing
                        ))
                        .frame(width: max(0, min(geometry.size.width,
                                                 geometry.size.width * winrateBlack)),
                               height: 20)
                    
                    // Midpoint line
                    Rectangle()
                        .fill(Color.secondary.opacity(0.5))
                        .frame(width: 1, height: 20)
                        .position(x: geometry.size.width / 2, y: 10)
                    
                    // Label
                    Text("黑")
                        .font(.caption2)
                        .foregroundColor(.white)
                        .position(x: 8, y: 10)
                    Text("白")
                        .font(.caption2)
                        .foregroundColor(.black)
                        .position(x: geometry.size.width - 8, y: 10)
                }
            }
            .frame(height: 20)
        }
        .padding(8)
        .background(Color(nsColor: .windowBackgroundColor).opacity(0.5))
        .cornerRadius(8)
    }
}
```

- [ ] **Step 2: Write ScoreLeadView.swift**

```swift
import SwiftUI

struct ScoreLeadView: View {
    let lead: Double // + = black leads, - = white leads
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("目差形势")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(leadText)
                    .font(.title3)
                    .fontWeight(.semibold)
                    .monospacedDigit()
            }
            
            GeometryReader { geometry in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4)
                        .fill(Color.gray.opacity(0.2))
                        .frame(height: 20)
                    
                    // Clamp lead to +/-20 for display
                    let clamped = max(-20, min(20, lead))
                    let fillWidth = ((clamped + 20) / 40.0) * geometry.size.width
                    
                    RoundedRectangle(cornerRadius: 4)
                        .fill(LinearGradient(
                            colors: [.black, .gray, .white],
                            startPoint: .leading,
                            endPoint: .trailing
                        ))
                        .frame(width: max(0, fillWidth), height: 20)
                    
                    // Center line
                    Rectangle()
                        .fill(Color.secondary.opacity(0.5))
                        .frame(width: 1, height: 20)
                        .position(x: geometry.size.width / 2, y: 10)
                    
                    Text("黑优")
                        .font(.caption2)
                        .foregroundColor(.white)
                        .position(x: 12, y: 10)
                    Text("白优")
                        .font(.caption2)
                        .foregroundColor(.black)
                        .position(x: geometry.size.width - 12, y: 10)
                }
            }
            .frame(height: 20)
        }
        .padding(8)
        .background(Color(nsColor: .windowBackgroundColor).opacity(0.5))
        .cornerRadius(8)
    }
    
    private var leadText: String {
        let absLead = abs(lead)
        if absLead < 0.1 { return "双方均势" }
        if lead > 0 { return "黑+\(String(format: "%.1f", absLead))目" }
        return "白+\(String(format: "%.1f", absLead))目"
    }
}
```

- [ ] **Step 3: Write AnalysisPanel.swift**

```swift
import SwiftUI

struct AnalysisPanel: View {
    let viewModel: GameViewModel
    
    var body: some View {
        VStack(spacing: 12) {
            // Current turn info
            HStack {
                Circle()
                    .fill(viewModel.currentPlayer == "b" ? Color.black : Color.white)
                    .frame(width: 16, height: 16)
                    .overlay(Circle().stroke(Color.gray, lineWidth: 1))
                Text("\(viewModel.currentPlayer == "b" ? "黑棋" : "白棋") 第\(viewModel.moveCount)手")
                    .font(.headline)
            }
            .padding(.top, 8)
            
            Divider()
            
            WinrateBar(winrateBlack: viewModel.winrateBlack)
            
            ScoreLeadView(lead: viewModel.lead)
            
            Divider()
            
            // Captures
            HStack {
                Text("提子")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text("黑 \(viewModel.capturesBlack)  ·  白 \(viewModel.capturesWhite)")
                    .font(.subheadline)
            }
            
            Spacer()
        }
        .padding()
        .frame(width: 200)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
```

- [ ] **Step 4: Build check**

---

### Task 6: Toolbar (GameToolbar + LevelPicker)

**Files:**
- Create: `KataGoGo/Views/Toolbar/LevelPicker.swift`
- Create: `KataGoGo/Views/Toolbar/GameToolbar.swift`

- [ ] **Step 1: Write LevelPicker.swift**

```swift
import SwiftUI

struct LevelPicker: View {
    let viewModel: GameViewModel
    
    let levels = [
        (0, "9级"),
        (1, "7级~8级"),
        (2, "4级~6级"),
        (3, "1级~3级"),
        (4, "1段~3段"),
        (5, "4段~6段"),
        (6, "7段以上"),
    ]
    
    @State private var selectedLevel: Int = 4
    
    var body: some View {
        Picker("AI 棋力", selection: $selectedLevel) {
            ForEach(levels, id: \.0) { id, label in
                Text(label).tag(id)
            }
        }
        .pickerStyle(.menu)
        .frame(width: 120)
        .onChange(of: selectedLevel) { _, newValue in
            viewModel.setLevel(newValue)
        }
    }
}
```

- [ ] **Step 2: Write GameToolbar.swift**

```swift
import SwiftUI

struct GameToolbar: View {
    let viewModel: GameViewModel
    
    var body: some View {
        HStack(spacing: 12) {
            Button("Pass") {
                viewModel.pass()
            }
            .disabled(viewModel.phase != .playing)
            
            Button("悔棋") {
                viewModel.undo()
            }
            .disabled(viewModel.phase != .playing)
            .keyboardShortcut("z", modifiers: .command)
            
            Button("数目") {
                _ = viewModel.finalScore()
            }
            .disabled(viewModel.phase != .playing)
            .keyboardShortcut("s", modifiers: .command)
            
            Divider()
                .frame(height: 20)
            
            LevelPicker(viewModel: viewModel)
            
            Spacer()
            
            if viewModel.phase == .waiting {
                ProgressView()
                    .scaleEffect(0.7)
                    .frame(width: 20, height: 20)
                Text("AI 思考中...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            if viewModel.phase == .connecting {
                ProgressView()
                    .scaleEffect(0.7)
                Text("启动引擎...")
                    .font(.caption)
            }
        }
        .padding(.horizontal)
        .padding(.vertical, 6)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
```

- [ ] **Step 3: Build check**

---

### Task 7: ContentView (main window layout)

**Files:**
- Modify: `KataGoGo/Views/ContentView.swift`

- [ ] **Step 1: Write ContentView.swift**

```swift
import SwiftUI

struct ContentView: View {
    
    @State private var viewModel = GameViewModel()
    @State private var showErrorAlert = false
    
    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            GameToolbar(viewModel: viewModel)
            Divider()
            
            // Main content: 3-column layout
            HStack(spacing: 0) {
                // Left: Mini-map
                MiniMapView(viewModel: viewModel)
                    .padding()
                    .frame(width: 200)
                
                Divider()
                
                // Center: Board
                BoardCanvas(viewModel: viewModel)
                    .frame(minWidth: 400, idealWidth: 600, maxWidth: .infinity,
                           minHeight: 400, idealHeight: 600, maxHeight: .infinity)
                
                Divider()
                
                // Right: Analysis panel
                AnalysisPanel(viewModel: viewModel)
                    .frame(width: 200)
            }
        }
        .onAppear {
            viewModel.startGame()
        }
        .onChange(of: viewModel.errorMessage) { _, newValue in
            showErrorAlert = newValue != nil
        }
        .alert("错误", isPresented: $showErrorAlert) {
            Button("确定", role: .cancel) {
                viewModel.errorMessage = nil
            }
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
        .frame(minWidth: 800, minHeight: 500)
    }
}
```

- [ ] **Step 2: Write KataGoGoApp.swift**

```swift
import SwiftUI

@main
struct KataGoGoApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
        .windowResizability(.contentMinSize)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("新对局") {
                    NotificationCenter.default.post(name: .newGame, object: nil)
                }
                .keyboardShortcut("n", modifiers: .command)
            }
        }
    }
}

extension Notification.Name {
    static let newGame = Notification.Name("com.katagogo.newGame")
}
```

- [ ] **Step 3: Final build and test**

Build (⌘B). If everything compiles:
- Set up your KataGo paths in AppState.swift
- Run (⌘R)
- You should see the 3-column layout, click the board to play

---

### Phase 1B Delivery Checklist

- [ ] Xcode project created with all source files
- [ ] Rust build integrated (Run Script phase)
- [ ] App starts, shows 3-column layout
- [ ] Click board → stone appears → AI responds
- [ ] WinrateBar updates after AI move
- [ ] ScoreLeadView shows 目差
- [ ] MiniMap shows move numbers
- [ ] Undo works (⌘Z)
- [ ] Level picker changes AI strength
- [ ] Dark mode looks reasonable
