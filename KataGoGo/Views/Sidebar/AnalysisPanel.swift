import SwiftUI
struct AnalysisPanel: View {
    @ObservedObject var viewModel: GameViewModel
    var body: some View {
        ScrollView(.vertical) {
            VStack(alignment: .leading, spacing: 10) {
                VStack(alignment: .leading, spacing: 12) {
                    PanelHeader(title: viewModel.isReviewMode ? "复盘分析" : "AI 分析", systemImage: viewModel.isReviewMode ? "clock.arrow.circlepath" : "brain.head.profile", trailingSystemImage: nil)
                    HStack(alignment: .center, spacing: 10) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("形势判断")
                                .font(.footnote)
                                .foregroundStyle(AppTheme.secondaryText)
                            Text(positionBadge)
                                .font(.title3.weight(.semibold))
                                .foregroundStyle(positionTint)
                        }
                        Spacer()
                        InlineStatusPill(title: currentTurnText, systemImage: nil, tint: AppTheme.teal)
                    }
                    Divider()
                    MetricRow(title: "贴目", value: "7.5 目")
                    MetricRow(title: "胜率", value: winrateSummary, accent: AppTheme.text)
                    MetricRow(title: "目差", value: leadSummary, accent: positionTint)
                }
                .padding(AppTheme.Metrics.panelPadding)
                .panelStyle()

                WinrateBar(winrateBlack: viewModel.winrateBlack)
                ScoreLeadView(leadBlack: viewModel.leadBlack)

                VStack(alignment: .leading, spacing: 10) {
                    PanelHeader(title: "引擎信息", systemImage: "cpu", trailingSystemImage: nil)
                    ModelInfoRow(title: "模型", value: viewModel.modelName, accent: AppTheme.text)
                    MetricRow(title: "让子", value: viewModel.handicapLabel, accent: AppTheme.teal)
                    MetricRow(title: "思考时间", value: thinkTimeText, accent: AppTheme.teal)
                    MetricRow(title: "思考步骤", value: thinkStepsText, accent: AppTheme.teal)
                    MetricRow(title: "最后落点", value: lastMoveText, accent: AppTheme.teal)
                    MetricRow(title: "评估准确率", value: accuracyText, accent: AppTheme.teal)
                }
                .padding(AppTheme.Metrics.panelPadding)
                .panelStyle()
            }
        }
        .frame(maxWidth: .infinity)
    }

    private var lastMoveText: String {
        guard let lastMove = viewModel.lastMove else { return "-" }
        let columns = Array("ABCDEFGHJKLMNOPQRST")
        guard lastMove.col >= 0, lastMove.col < columns.count else { return "-" }
        return "\(columns[lastMove.col])\(19 - lastMove.row)"
    }

    private var accuracyText: String {
        String(format: "%.0f%%", viewModel.evaluationAccuracy * 100)
    }

    private var thinkTimeText: String {
        if viewModel.phase == .waiting {
            return "思考中"
        }
        guard let duration = viewModel.lastThinkDuration else { return "-" }
        return duration < 1
            ? String(format: "%.0f ms", duration * 1000)
            : String(format: "%.1f s", duration)
    }

    private var thinkStepsText: String {
        if viewModel.phase == .waiting {
            return "思考中"
        }
        guard viewModel.moveCount > 0 else { return "-" }
        return "\(viewModel.completedThinkSteps)/\(viewModel.maxThinkSteps)"
    }

    private var winrateTitle: String {
        "胜率（黑棋 / 白棋）"
    }

    private var winrateSummary: String {
        String(format: "%.1f%% / %.1f%%", viewModel.winrateBlack * 100, viewModel.winrateWhite * 100)
    }

    private var leadSummary: String {
        guard viewModel.leadBlack.isFinite else { return "-" }
        return String(format: "黑棋 %+.1f", viewModel.leadBlack)
    }

    private var currentTurnText: String {
        viewModel.currentPlayer == "b" ? "黑方行棋" : "白方行棋"
    }

    private var positionBadge: String {
        if abs(viewModel.leadBlack) < 1.0 { return "局势接近" }
        return viewModel.leadBlack > 0 ? "黑棋优势" : "白棋优势"
    }

    private var positionTint: Color {
        abs(viewModel.leadBlack) < 1.0 ? AppTheme.secondaryText : AppTheme.teal
    }
}

private struct ModelInfoRow: View {
    let title: String
    let value: String
    var accent: Color = AppTheme.text

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.footnote)
                .foregroundStyle(AppTheme.secondaryText)
            Text(value)
                .font(.footnote.weight(.semibold))
                .foregroundStyle(accent)
                .lineLimit(2)
                .truncationMode(.middle)
                .frame(maxWidth: .infinity, alignment: .leading)
                .textSelection(.enabled)
        }
    }
}
