import SwiftUI
struct BoardCanvas: NSViewRepresentable {
    let viewModel: GameViewModel
    let boardVersion: Int
    let suggestionCount: Int
    let showsLabels: Bool
    let territoryCount: Int

    func makeNSView(context: Context) -> BoardNSView { BoardNSView() }
    func updateNSView(_ nsView: BoardNSView, context: Context) {
        nsView.viewModel = viewModel; nsView.needsDisplay = true
    }
}
class BoardNSView: NSView {
    weak var viewModel: GameViewModel?
    private let renderer = BoardRenderer(boardSize: 19)
    
    override func draw(_ dirtyRect: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext, let vm = viewModel else { return }
        var stones: [(Int, Int, Bool)] = []
        for r in 0..<19 { for c in 0..<19 { if let b = vm.board[r][c] { stones.append((c, r, b)) } } }
        renderer.drawBoard(
            context: ctx,
            size: bounds.size,
            stones: stones,
            lastMove: vm.lastMove,
            moveLabels: vm.showsMoveLabelsOnMainBoard ? vm.moveLabels : [],
            showCoordinates: vm.showsCoordinatesOnMainBoard,
            suggestions: vm.isShowingSuggestions ? vm.moveSuggestions : [],
            territory: vm.territory
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
