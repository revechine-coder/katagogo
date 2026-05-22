import SwiftUI
struct BoardCanvas: NSViewRepresentable {
    let viewModel: GameViewModel
    let renderState: BoardRenderState
    let showsLabels: Bool
    let showsCoordinates: Bool
    let isShowingSuggestions: Bool
    let territoryCount: Int

    func makeNSView(context: Context) -> BoardNSView { BoardNSView() }
    func updateNSView(_ nsView: BoardNSView, context: Context) {
        nsView.viewModel = viewModel
        nsView.update(
            renderState: renderState,
            showsLabels: showsLabels,
            showsCoordinates: showsCoordinates,
            isShowingSuggestions: isShowingSuggestions
        )
    }
}
class BoardNSView: NSView {
    weak var viewModel: GameViewModel?
    private var renderState: BoardRenderState = .empty
    private var showsLabels = false
    private var showsCoordinates = false
    private var isShowingSuggestions = false
    private let renderer = BoardRenderer(boardSize: 19)
    private var breathingTimer: Timer?

    deinit {
        breathingTimer?.invalidate()
    }

    func update(
        renderState newState: BoardRenderState,
        showsLabels newShowsLabels: Bool,
        showsCoordinates newShowsCoordinates: Bool,
        isShowingSuggestions newIsShowingSuggestions: Bool
    ) {
        assert(Thread.isMainThread)
        let oldState = renderState
        let needsFullRedraw = bounds.isEmpty ||
            oldState.boardSize != newState.boardSize ||
            showsLabels != newShowsLabels ||
            showsCoordinates != newShowsCoordinates ||
            oldState.moveLabels != newState.moveLabels ||
            oldState.territory.count != newState.territory.count

        renderState = newState
        showsLabels = newShowsLabels
        showsCoordinates = newShowsCoordinates
        isShowingSuggestions = newIsShowingSuggestions
        updateBreathingTimer()

        if needsFullRedraw {
            needsDisplay = true
        } else {
            markDirtyIntersections(from: oldState, to: newState)
        }
    }
    
    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        renderer.drawBoard(
            context: ctx,
            size: bounds.size,
            stones: renderState.stones.map { ($0.col, $0.row, $0.isBlack) },
            lastMove: renderState.lastMove,
            moveLabels: showsLabels ? renderState.moveLabels.map { ($0.col, $0.row, $0.moveNumber) } : [],
            showCoordinates: showsCoordinates,
            suggestions: isShowingSuggestions ? renderState.suggestions.map { ($0.col, $0.row, $0.winrate, $0.lead, $0.visits, $0.order) } : [],
            territory: renderState.territory
        )
    }
    
    override func mouseDown(with event: NSEvent) {
        guard let vm = viewModel else { return }
        let p = convert(event.locationInWindow, from: nil)
        let side = min(bounds.width, bounds.height)
        let origin = CGPoint(x: (bounds.width - side) / 2, y: (bounds.height - side) / 2)
        let pad = side * 0.058
        let gridSize = (side - 2 * pad) / 18
        let boardMinX = origin.x + pad
        let boardMinY = origin.y + pad
        let boardMaxX = boardMinX + gridSize * 18
        let boardMaxY = boardMinY + gridSize * 18
        guard p.x >= boardMinX - gridSize * 0.5,
              p.x <= boardMaxX + gridSize * 0.5,
              p.y >= boardMinY - gridSize * 0.5,
              p.y <= boardMaxY + gridSize * 0.5 else { return }
        let c = Int(round((p.x - boardMinX) / gridSize))
        let r = Int(round((p.y - boardMinY) / gridSize))
        guard c >= 0, c < 19, r >= 0, r < 19 else { return }
        vm.play(at: c, row: r)
    }

    private func markDirtyIntersections(from oldState: BoardRenderState, to newState: BoardRenderState) {
        var dirtyPoints = Set<BoardPointKey>()
        let oldStones = Dictionary(uniqueKeysWithValues: oldState.stones.map { (BoardPointKey(col: $0.col, row: $0.row), $0.isBlack) })
        let newStones = Dictionary(uniqueKeysWithValues: newState.stones.map { (BoardPointKey(col: $0.col, row: $0.row), $0.isBlack) })
        for key in Set(oldStones.keys).union(newStones.keys) where oldStones[key] != newStones[key] {
            dirtyPoints.insert(key)
        }

        let oldSuggestions = Dictionary(uniqueKeysWithValues: oldState.suggestions.map { (BoardPointKey(col: $0.col, row: $0.row), $0) })
        let newSuggestions = Dictionary(uniqueKeysWithValues: newState.suggestions.map { (BoardPointKey(col: $0.col, row: $0.row), $0) })
        for key in Set(oldSuggestions.keys).union(newSuggestions.keys) where oldSuggestions[key] != newSuggestions[key] {
            dirtyPoints.insert(key)
        }

        if let lastMove = oldState.lastMove {
            dirtyPoints.insert(BoardPointKey(col: lastMove.col, row: lastMove.row))
        }
        if let lastMove = newState.lastMove {
            dirtyPoints.insert(BoardPointKey(col: lastMove.col, row: lastMove.row))
        }

        guard !dirtyPoints.isEmpty else { return }
        for point in dirtyPoints {
            setNeedsDisplay(intersectionRect(col: point.col, row: point.row))
        }
    }

    private func intersectionRect(col: Int, row: Int) -> NSRect {
        let side = min(bounds.width, bounds.height)
        let origin = CGPoint(x: (bounds.width - side) / 2, y: (bounds.height - side) / 2)
        let padding = side * 0.058
        let gridSize = (side - 2 * padding) / CGFloat(max(renderState.boardSize - 1, 1))
        let x = origin.x + padding + CGFloat(col) * gridSize
        let y = origin.y + padding + CGFloat(row) * gridSize
        let radius = gridSize * 0.78
        return NSRect(x: x - radius, y: y - radius, width: radius * 2, height: radius * 2)
    }

    private func updateBreathingTimer() {
        let shouldAnimate = isShowingSuggestions && !renderState.suggestions.isEmpty
        if shouldAnimate, breathingTimer == nil {
            breathingTimer = Timer.scheduledTimer(withTimeInterval: 1.0 / 24.0, repeats: true) { [weak self] _ in
                guard let self else { return }
                for suggestion in self.renderState.suggestions {
                    self.setNeedsDisplay(self.intersectionRect(col: suggestion.col, row: suggestion.row))
                }
            }
        } else if !shouldAnimate {
            breathingTimer?.invalidate()
            breathingTimer = nil
        }
    }
    
    override func resetCursorRects() {
        addCursorRect(bounds, cursor: .pointingHand)
    }
    
    override var acceptsFirstResponder: Bool { true }

    override func keyDown(with event: NSEvent) {
        guard let vm = viewModel, vm.isReviewMode else {
            super.keyDown(with: event)
            return
        }
        switch Int(event.keyCode) {
        case 123: // Left arrow
            vm.stepBackward()
        case 124: // Right arrow
            vm.stepForward()
        default:
            super.keyDown(with: event)
        }
    }
}

private struct BoardPointKey: Hashable {
    let col: Int
    let row: Int
}
