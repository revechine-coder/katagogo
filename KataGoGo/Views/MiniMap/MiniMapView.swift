import SwiftUI
struct MiniMapView: View {
    let viewModel: GameViewModel
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("棋局总览")
                .font(.system(.headline, design: .rounded, weight: .semibold))
                .foregroundStyle(AppTheme.text)
            MiniMapCanvas(viewModel: viewModel)
                .frame(width: 176, height: 176)
                .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 7, style: .continuous)
                        .stroke(AppTheme.hairline, lineWidth: 1)
                )
                .shadow(color: .black.opacity(0.08), radius: 8, x: 0, y: 4)
            HStack {
                MetricRow(title: "手数", value: "\(viewModel.moveCount)")
                Divider().frame(height: 18)
                MetricRow(title: "提子", value: "\(viewModel.capturesBlack):\(viewModel.capturesWhite)")
            }
        }
        .padding(14)
        .panelStyle()
    }
}

struct MoveTimelineView: View {
    let viewModel: GameViewModel
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("落子记录")
                .font(.system(.headline, design: .rounded, weight: .semibold))
                .foregroundStyle(AppTheme.text)
            if viewModel.moveLabels.isEmpty {
                Text("开局后将在这里显示最近落点。")
                    .font(.caption)
                    .foregroundStyle(AppTheme.secondaryText)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 6)
            } else {
                ScrollView(.vertical) {
                    LazyVStack(spacing: 2) {
                        ForEach(allMoves, id: \.moveNumber) { move in
                            HStack(spacing: 8) {
                                Text("\(move.moveNumber)")
                                    .font(.caption.monospacedDigit())
                                    .foregroundStyle(AppTheme.secondaryText)
                                    .frame(width: 32, alignment: .leading)
                                Text(vertex(col: move.col, row: move.row))
                                    .font(.system(.subheadline, design: .rounded, weight: .semibold))
                                    .foregroundStyle(AppTheme.text)
                                Spacer()
                            }
                            .padding(.vertical, 4)
                            .padding(.horizontal, 6)
                            .background(move.moveNumber == viewModel.moveCount ? AppTheme.tealSoft.opacity(0.45) : Color.clear)
                            .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
                        }
                    }
                }
                .frame(minHeight: 120, idealHeight: 190, maxHeight: 260)
                .scrollIndicators(.visible)
            }
        }
        .padding(14)
        .panelStyle()
    }
    
    private var allMoves: [(col: Int, row: Int, moveNumber: Int)] {
        Array(viewModel.moveLabels.reversed())
    }
    
    private func vertex(col: Int, row: Int) -> String {
        let columns = Array("ABCDEFGHJKLMNOPQRST")
        guard col >= 0, col < columns.count else { return "-" }
        return "\(columns[col])\(19 - row)"
    }
}
struct MiniMapCanvas: NSViewRepresentable {
    let viewModel: GameViewModel
    func makeNSView(context: Context) -> MiniMapNSView { MiniMapNSView() }
    func updateNSView(_ n: MiniMapNSView, context: Context) { n.viewModel = viewModel; n.needsDisplay = true }
}
class MiniMapNSView: NSView {
    weak var viewModel: GameViewModel?
    private let r = BoardRenderer(boardSize: 19)
    override func draw(_ d: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext, let vm = viewModel else { return }
        var stones: [(Int, Int, Bool)] = []
        for r in 0..<19 { for c in 0..<19 { if let b = vm.board[r][c] { stones.append((c, r, b)) } } }
        self.r.drawBoard(context: ctx, size: bounds.size, stones: stones, lastMove: nil, moveLabels: vm.moveLabels, showCoordinates: false, miniMap: true)
    }
}
