import SwiftUI

struct ContentView: View {
    @StateObject private var viewModel = GameViewModel()
    @State private var showErrorAlert = false

    var body: some View {
        VStack(spacing: 0) {
            GameToolbar(viewModel: viewModel)
                .zIndex(10)
            mainLayout
                .zIndex(0)
        }
        .background(AppTheme.appBackground)
        .onChange(of: viewModel.errorMessage) {
            showErrorAlert = viewModel.errorMessage != nil
        }
        .alert("错误", isPresented: $showErrorAlert) {
            Button("确定", role: .cancel) { viewModel.errorMessage = nil }
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
        .frame(minWidth: 940, minHeight: 620)
    }
    
    private var mainLayout: some View {
        GeometryReader { proxy in
            let width = proxy.size.width
            let spacing: CGFloat = width < 920 ? 12 : 18
            let padding: CGFloat = width < 920 ? 12 : 18
            let leftWidth = min(width < 920 ? 196 : 224, max(174, width * 0.22))
            let rightWidth = min(width < 920 ? 248 : 284, max(220, width * 0.27))

            HStack(alignment: .top, spacing: spacing) {
                leftColumn
                    .frame(width: leftWidth)
                centerColumn
                VStack(spacing: 14) {
                    AnalysisPanel(viewModel: viewModel)
                    if viewModel.isReviewMode {
                        ReviewControlPanel(viewModel: viewModel)
                    }
                }
                .frame(width: rightWidth)
            }
            .padding(padding)
            .frame(width: proxy.size.width, height: proxy.size.height, alignment: .top)
        }
    }
    
    private var leftColumn: some View {
        VStack(spacing: 14) {
            MiniMapView(viewModel: viewModel)
            CaptureStatsView(viewModel: viewModel)
            MoveTimelineView(viewModel: viewModel)
            Spacer(minLength: 0)
        }
    }

    private var centerColumn: some View {
        VStack(spacing: 12) {
            BoardStatusStrip(viewModel: viewModel)
                .frame(maxWidth: .infinity)
            boardStage
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    
    private var boardStage: some View {
        let shape = RoundedRectangle(cornerRadius: AppTheme.panelRadius, style: .continuous)

        return ZStack {
            BoardCanvas(viewModel: viewModel, boardVersion: viewModel.boardVersion, suggestionCount: viewModel.moveSuggestions.count, showsLabels: viewModel.showsMoveLabelsOnMainBoard, territoryCount: viewModel.territory.count)
                .padding(AppTheme.Metrics.boardPadding)
        }
        .background(AppTheme.boardSurface, in: shape)
        .background(.regularMaterial, in: shape)
        .overlay(shape.stroke(AppTheme.hairline, lineWidth: 1))
        .shadow(color: .black.opacity(0.18), radius: 15, x: 0, y: 10)
        .shadow(color: .black.opacity(0.05), radius: 2, x: 0, y: 1)
        .aspectRatio(1, contentMode: .fit)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

private struct BoardStatusStrip: View {
    @ObservedObject var viewModel: GameViewModel
    private let stripHeight: CGFloat = 38
    private let phaseWidth: CGFloat = 88
    private let turnWidth: CGFloat = 76
    private let metricWidth: CGFloat = 76

    var body: some View {
        HStack(spacing: AppTheme.Spacing.sm) {
            InlineStatusPill(title: phaseText, systemImage: phaseIcon, tint: phaseColor)
                .frame(width: phaseWidth)

            Divider().frame(height: 20)

            HStack(spacing: 6) {
                StoneToken(isBlack: viewModel.currentPlayer == "b", size: 13)
                Text(currentTurnText)
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(AppTheme.text)
            }
            .frame(width: turnWidth, alignment: .leading)

            Spacer(minLength: AppTheme.Spacing.sm)

            metric("手数", "\(viewModel.moveCount)")
            metric("最后", lastMoveText)
            metric("胜率", String(format: "%.1f%%", viewModel.winrateBlack * 100))
        }
        .padding(.horizontal, AppTheme.Spacing.md)
        .padding(.vertical, 8)
        .panelStyle(.glass)
        .frame(maxWidth: .infinity, minHeight: stripHeight, maxHeight: stripHeight)
        .transaction { transaction in
            transaction.animation = nil
        }
    }

    private func metric(_ title: String, _ value: String) -> some View {
        HStack(spacing: 5) {
            Text(title)
                .font(.caption)
                .foregroundStyle(AppTheme.secondaryText)
            Text(value)
                .font(.footnote.monospacedDigit().weight(.semibold))
                .foregroundStyle(AppTheme.text)
                .lineLimit(1)
        }
        .frame(width: metricWidth, alignment: .trailing)
    }

    private var currentTurnText: String {
        viewModel.currentPlayer == "b" ? "黑方行棋" : "白方行棋"
    }

    private var lastMoveText: String {
        guard let lastMove = viewModel.lastMove else { return "-" }
        let columns = Array("ABCDEFGHJKLMNOPQRST")
        guard lastMove.col >= 0, lastMove.col < columns.count else { return "-" }
        return "\(columns[lastMove.col])\(19 - lastMove.row)"
    }

    private var phaseText: String {
        switch viewModel.phase {
        case .idle: "未开始"
        case .connecting: "准备中"
        case .playing: viewModel.currentPlayer == viewModel.humanColor ? "请落子" : "引擎就绪"
        case .paused: "已暂停"
        case .waiting: "AI 思考"
        case .finished: "对局结束"
        }
    }

    private var phaseIcon: String {
        switch viewModel.phase {
        case .idle: "circle"
        case .connecting: "bolt.horizontal"
        case .playing: "hand.point.up.left"
        case .paused: "pause.fill"
        case .waiting: "brain.head.profile"
        case .finished: "flag.checkered"
        }
    }

    private var phaseColor: Color {
        switch viewModel.phase {
        case .playing, .waiting: AppTheme.teal
        case .finished: AppTheme.warning
        default: AppTheme.secondaryText
        }
    }
}

private struct ReviewControlPanel: View {
    @ObservedObject var viewModel: GameViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            PanelHeader(title: "复盘控制", systemImage: "clock.arrow.circlepath", trailingSystemImage: nil)

            HStack(spacing: 2) {
                navButton(icon: "backward.end.fill", label: "开始复盘", action: { viewModel.jumpToMove(0) })
                navButton(icon: "chevron.left", label: "上一手", isDisabled: viewModel.replayTargetMove == 0, action: { viewModel.stepBackward() })
                Spacer()
                Text("\(viewModel.replayTargetMove)/\(viewModel.totalRecordedMoves)")
                    .font(.subheadline.weight(.semibold))
                    .monospacedDigit()
                    .foregroundStyle(AppTheme.text)
                Spacer()
                navButton(icon: "chevron.right", label: "下一手", isDisabled: viewModel.replayTargetMove >= viewModel.totalRecordedMoves, action: { viewModel.stepForward() })
                navButton(icon: "forward.end.fill", label: "跳到最新", isDisabled: viewModel.replayTargetMove >= viewModel.totalRecordedMoves, action: { viewModel.jumpToMove(viewModel.totalRecordedMoves) })
            }

            HStack(spacing: 6) {
                Button {
                    viewModel.resetReviewPosition()
                } label: {
                    HStack(spacing: 4) {
                        Image(systemName: "arrow.clockwise").font(.system(size: 10, weight: .bold))
                        Text("复位").font(.footnote.weight(.semibold))
                    }
                    .foregroundStyle(AppTheme.secondaryText)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 5)
                    .background(AppTheme.controlTrack)
                    .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
                }
                .buttonStyle(.plain)

                Button {
                    viewModel.continueFromReview()
                } label: {
                    HStack(spacing: 4) {
                        Image(systemName: "play.fill").font(.system(size: 10))
                        Text("继续对局").font(.footnote.weight(.semibold))
                    }
                    .foregroundStyle(.white)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 5)
                    .background(AppTheme.teal)
                    .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }

    private func navButton(
        icon: String,
        label: String,
        isDisabled: Bool = false,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(isDisabled ? AppTheme.tertiaryText : AppTheme.text)
                .frame(width: 30, height: 26)
                .background(
                    RoundedRectangle(cornerRadius: 5, style: .continuous)
                        .fill(isDisabled ? Color.clear : AppTheme.controlTrack)
                )
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(isDisabled)
        .help(label)
    }
}
