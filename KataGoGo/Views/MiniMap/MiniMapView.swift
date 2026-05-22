import SwiftUI
struct MiniMapView: View {
    @ObservedObject var viewModel: GameViewModel
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            PanelHeader(title: "对局概览", trailingSystemImage: nil)

            ZStack(alignment: .topTrailing) {
                MiniMapCanvas(viewModel: viewModel, renderState: viewModel.boardRenderState)
                    .frame(maxWidth: .infinity)
                    .aspectRatio(1, contentMode: .fit)
                    .clipShape(RoundedRectangle(cornerRadius: 7, style: .continuous))
                    .help("点击切换主棋盘手数标签显示")

                minimapHint
                    .padding(6)
            }
            .overlay(
                RoundedRectangle(cornerRadius: 7, style: .continuous)
                    .stroke(viewModel.showsMoveLabelsOnMainBoard ? AppTheme.teal.opacity(0.78) : AppTheme.hairline, lineWidth: viewModel.showsMoveLabelsOnMainBoard ? 1.5 : 1)
            )
            .shadow(color: .black.opacity(0.08), radius: 8, x: 0, y: 4)

            HStack(spacing: 6) {
                Image(systemName: viewModel.showsCoordinatesOnMainBoard ? "grid" : "grid.off")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(viewModel.showsCoordinatesOnMainBoard ? AppTheme.tealDark : AppTheme.tertiaryText)
                Button {
                    viewModel.showsCoordinatesOnMainBoard.toggle()
                    viewModel.boardVersion &+= 1
                } label: {
                    Text("坐标")
                        .font(.caption)
                        .foregroundStyle(viewModel.showsCoordinatesOnMainBoard ? AppTheme.text : AppTheme.secondaryText)
                }
                .buttonStyle(.plain)
                .help(viewModel.showsCoordinatesOnMainBoard ? "隐藏主棋盘坐标" : "显示主棋盘坐标")

                Spacer()

                HStack(spacing: 4) {
                    Circle()
                        .fill(viewModel.showsMoveLabelsOnMainBoard ? AppTheme.teal : AppTheme.tertiaryText)
                        .frame(width: 5, height: 5)
                    Text("手数标签")
                        .font(.caption)
                        .foregroundStyle(viewModel.showsMoveLabelsOnMainBoard ? AppTheme.text : AppTheme.secondaryText)
                }
            }

            VStack(spacing: 0) {
                HStack {
                    Text("对局类型")
                    Spacer()
                    Text("贴目对局（7.5目）")
                        .fontWeight(.semibold)
                }
                .padding(.bottom, 8)

                Divider()

                HStack {
                    SummaryMetric(title: "当前步数", value: "\(viewModel.moveCount)")
                    Divider().frame(height: 34)
                    SummaryMetric(title: "总用时", value: viewModel.totalElapsedText)
                }
                .padding(.top, 8)
            }
            .font(.footnote)
            .foregroundStyle(AppTheme.secondaryText)
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }

    private var minimapHint: some View {
        HStack(spacing: 3) {
            Image(systemName: "text.viewfinder")
                .font(.system(size: 9, weight: .semibold))
            Text("点击切换标签")
                .font(.system(size: 9, weight: .medium))
        }
        .foregroundStyle(AppTheme.tertiaryText)
        .padding(.horizontal, 6)
        .padding(.vertical, 3)
        .background(AppTheme.surface.opacity(0.82), in: RoundedRectangle(cornerRadius: 4, style: .continuous))
        .allowsHitTesting(false)
    }
}

struct MoveTimelineView: View {
    @ObservedObject var viewModel: GameViewModel
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            PanelHeader(title: viewModel.isReviewMode ? "复盘 · 第\(viewModel.replayTargetMove)/\(viewModel.totalRecordedMoves)手" : "落子记录")

            if viewModel.moveLabels.isEmpty {
                Text("开局后将在这里显示最近落点。")
                    .font(.footnote)
                    .foregroundStyle(AppTheme.secondaryText)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.vertical, 6)
            } else {
                ScrollView(.vertical) {
                    LazyVStack(spacing: 2) {
                        ForEach(allMoves, id: \.moveNumber) { move in
                            Button {
                                viewModel.jumpToMove(move.moveNumber)
                            } label: {
                                HStack(spacing: 8) {
                                    Text("\(move.moveNumber)")
                                        .font(.caption.monospacedDigit())
                                        .foregroundStyle(AppTheme.secondaryText)
                                        .frame(width: 34, alignment: .leading)
                                    StoneToken(isBlack: move.moveNumber % 2 == 1, size: 15)
                                    Text(vertex(col: move.col, row: move.row))
                                        .font(.subheadline.weight(.semibold))
                                        .foregroundStyle(AppTheme.text)
                                    Spacer()
                                    if viewModel.isReviewMode, let snapshot = reviewSnapshot(for: move.moveNumber) {
                                        Text(String(format: "%.1f%%", snapshot.winrateBlack * 100))
                                            .font(.caption.monospacedDigit())
                                            .foregroundStyle(snapshot.winrateBlack >= 0.5 ? AppTheme.tealDark : AppTheme.warning)
                                    } else {
                                        Text(moveTime(for: move.moveNumber))
                                            .font(.caption.monospacedDigit())
                                            .foregroundStyle(AppTheme.tertiaryText)
                                    }
                                }
                                .padding(.vertical, 4)
                                .padding(.horizontal, 6)
                                .background(rowHighlight(for: move.moveNumber))
                                .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
                            }
                            .buttonStyle(.plain)
                            .overlay(
                                RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous)
                                    .stroke(rowHighlightStroke(for: move.moveNumber), lineWidth: 1)
                            )
                        }
                    }
                }
                .frame(minHeight: 120, idealHeight: 190, maxHeight: viewModel.isReviewMode ? 200 : 260)
            }
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }

    private func rowHighlight(for moveNumber: Int) -> Color {
        if viewModel.isReviewMode {
            return moveNumber == viewModel.replayTargetMove ? AppTheme.tealSoft.opacity(0.72) : Color.clear
        }
        return moveNumber == viewModel.moveCount ? AppTheme.tealSoft.opacity(0.72) : Color.clear
    }

    private func rowHighlightStroke(for moveNumber: Int) -> Color {
        let isActive = viewModel.isReviewMode
            ? moveNumber == viewModel.replayTargetMove
            : moveNumber == viewModel.moveCount
        return isActive ? AppTheme.teal.opacity(0.75) : Color.clear
    }

    private func reviewSnapshot(for moveNumber: Int) -> MoveAnalysisSnapshot? {
        viewModel.moveAnalysisHistory.last { $0.moveNumber <= moveNumber }
    }
    
    private var allMoves: [(col: Int, row: Int, moveNumber: Int)] {
        Array(viewModel.moveLabels.reversed())
    }
    
    private func vertex(col: Int, row: Int) -> String {
        let columns = Array("ABCDEFGHJKLMNOPQRST")
        guard col >= 0, col < columns.count else { return "-" }
        return "\(columns[col])\(19 - row)"
    }

    private func moveTime(for moveNumber: Int) -> String {
        viewModel.moveElapsedText(for: moveNumber)
    }
}

struct CaptureStatsView: View {
    @ObservedObject var viewModel: GameViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            PanelHeader(title: "提子统计", trailingSystemImage: nil)

            HStack(spacing: 14) {
                CaptureNumber(title: "黑方提子", count: viewModel.capturesBlack, isBlack: true)
                Divider().frame(height: 42)
                CaptureNumber(title: "白方提子", count: viewModel.capturesWhite, isBlack: false)
            }
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }
}

private struct CaptureNumber: View {
    let title: String
    let count: Int
    let isBlack: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(spacing: 6) {
                StoneToken(isBlack: isBlack, size: 14)
                Text(title)
                    .font(.footnote)
                    .foregroundStyle(AppTheme.secondaryText)
            }
            Text("\(count)")
                .font(.title2.weight(.semibold))
                .monospacedDigit()
                .foregroundStyle(AppTheme.text)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct SummaryMetric: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .foregroundStyle(AppTheme.secondaryText)
            Text(value)
                .font(.subheadline.weight(.semibold))
                .monospacedDigit()
                .foregroundStyle(AppTheme.text)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct MiniMapCanvas: NSViewRepresentable {
    let viewModel: GameViewModel
    let renderState: BoardRenderState

    func makeNSView(context: Context) -> MiniMapNSView { MiniMapNSView() }
    func updateNSView(_ nsView: MiniMapNSView, context: Context) {
        nsView.viewModel = viewModel
        nsView.update(renderState: renderState)
    }
}
class MiniMapNSView: NSView {
    weak var viewModel: GameViewModel?
    private var renderState: BoardRenderState = .empty
    private let r = BoardRenderer(boardSize: 19)

    func update(renderState newState: BoardRenderState) {
        assert(Thread.isMainThread)
        let oldState = renderState
        renderState = newState
        if oldState.boardSize != newState.boardSize || oldState.moveLabels != newState.moveLabels {
            needsDisplay = true
        } else {
            markDirtyIntersections(from: oldState, to: newState)
        }
    }

    override func draw(_ d: NSRect) {
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }
        self.r.drawBoard(
            context: ctx,
            size: bounds.size,
            stones: renderState.stones.map { ($0.col, $0.row, $0.isBlack) },
            lastMove: nil,
            moveLabels: renderState.moveLabels.map { ($0.col, $0.row, $0.moveNumber) },
            showCoordinates: false,
            miniMap: true
        )
    }

    private func markDirtyIntersections(from oldState: BoardRenderState, to newState: BoardRenderState) {
        var dirtyPoints = Set<MiniMapPointKey>()
        let oldStones = Dictionary(uniqueKeysWithValues: oldState.stones.map { (MiniMapPointKey(col: $0.col, row: $0.row), $0.isBlack) })
        let newStones = Dictionary(uniqueKeysWithValues: newState.stones.map { (MiniMapPointKey(col: $0.col, row: $0.row), $0.isBlack) })
        for key in Set(oldStones.keys).union(newStones.keys) where oldStones[key] != newStones[key] {
            dirtyPoints.insert(key)
        }

        if let lastMove = oldState.lastMove {
            dirtyPoints.insert(MiniMapPointKey(col: lastMove.col, row: lastMove.row))
        }
        if let lastMove = newState.lastMove {
            dirtyPoints.insert(MiniMapPointKey(col: lastMove.col, row: lastMove.row))
        }

        guard !dirtyPoints.isEmpty else { return }
        for point in dirtyPoints {
            setNeedsDisplay(intersectionRect(col: point.col, row: point.row))
        }
    }

    private func intersectionRect(col: Int, row: Int) -> NSRect {
        let side = min(bounds.width, bounds.height)
        let origin = CGPoint(x: (bounds.width - side) / 2, y: (bounds.height - side) / 2)
        let padding = max(15, side * 0.09)
        let gridSize = (side - 2 * padding) / CGFloat(max(renderState.boardSize - 1, 1))
        let x = origin.x + padding + CGFloat(col) * gridSize
        let y = origin.y + padding + CGFloat(row) * gridSize
        let radius = gridSize * 0.9
        return NSRect(x: x - radius, y: y - radius, width: radius * 2, height: radius * 2)
    }

    override func mouseDown(with event: NSEvent) {
        let show = !(viewModel?.showsMoveLabelsOnMainBoard ?? false)
        viewModel?.showsMoveLabelsOnMainBoard = show
        viewModel?.showsCoordinatesOnMainBoard = show
        viewModel?.boardVersion &+= 1
    }

    override func resetCursorRects() {
        addCursorRect(bounds, cursor: .pointingHand)
    }
}

private struct MiniMapPointKey: Hashable {
    let col: Int
    let row: Int
}
