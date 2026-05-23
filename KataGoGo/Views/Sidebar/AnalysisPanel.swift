import SwiftUI
struct AnalysisPanel: View {
    @ObservedObject var viewModel: GameViewModel
    var body: some View {
        ScrollView(.vertical) {
            VStack(alignment: .leading, spacing: 12) {
                PanelHeader(title: viewModel.isReviewMode ? "复盘分析" : "AI 分析",
                            systemImage: viewModel.isReviewMode ? "clock.arrow.circlepath" : "brain.head.profile",
                            trailingSystemImage: nil)

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

                MetricRow(title: "贴目", value: "7.5 目")

                Divider()

                WinrateCompactBar(winrateBlack: viewModel.winrateBlack)

                Divider()

                ScoreLeadCompactBar(leadBlack: viewModel.leadBlack)

                Divider()

                engineInfoSection
            }
            .padding(AppTheme.Metrics.panelPadding)
        }
        .frame(maxWidth: .infinity)
        .panelStyle()
    }

    private var engineInfoSection: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("引擎信息")
                .font(.footnote.weight(.semibold))
                .foregroundStyle(AppTheme.secondaryText)

            HStack(spacing: 8) {
                engineMetric("模型", viewModel.modelName)
                engineMetric("让子", viewModel.handicapLabel)
            }
            HStack(spacing: 8) {
                engineMetric("思考时间", thinkTimeText)
                engineMetric("思考步骤", thinkStepsText)
            }
            HStack(spacing: 8) {
                engineMetric("最后落点", lastMoveText)
                engineMetric("评估准确率", accuracyText)
            }
        }
        .padding(10)
        .background(AppTheme.surface, in: RoundedRectangle(cornerRadius: 6, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 6, style: .continuous)
                .stroke(AppTheme.hairline, lineWidth: 1)
        )
    }

    private func engineMetric(_ title: String, _ value: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.caption)
                .foregroundStyle(AppTheme.secondaryText)
            Text(value)
                .font(.caption.weight(.semibold))
                .foregroundStyle(AppTheme.text)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
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
        if viewModel.phase == .waiting { return "思考中" }
        guard let duration = viewModel.lastThinkDuration else { return "-" }
        return duration < 1
            ? String(format: "%.0f ms", duration * 1000)
            : String(format: "%.1f s", duration)
    }

    private var thinkStepsText: String {
        if viewModel.phase == .waiting { return "思考中" }
        guard viewModel.moveCount > 0 else { return "-" }
        return "\(viewModel.completedThinkSteps)/\(viewModel.maxThinkSteps)"
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

private struct WinrateCompactBar: View {
    let winrateBlack: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("胜率")
                .font(.footnote)
                .foregroundStyle(AppTheme.secondaryText)

            GeometryReader { g in
                HStack(spacing: 0) {
                    let clamped = max(0.02, min(0.98, winrateBlack))
                    let blackWidth = g.size.width * clamped
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .fill(LinearGradient(
                            colors: [Color(red: 0.12, green: 0.13, blue: 0.14), .black],
                            startPoint: .leading, endPoint: .trailing
                        ))
                        .frame(width: blackWidth, height: AppTheme.Metrics.chartHeight)
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .fill(Color.white.opacity(0.88))
                        .frame(width: g.size.width - blackWidth, height: AppTheme.Metrics.chartHeight)
                }
            }
            .frame(height: AppTheme.Metrics.chartHeight)

            HStack {
                Text(String(format: "黑 %.1f%%", winrateBlack * 100))
                    .font(.caption.monospacedDigit().weight(.semibold))
                    .foregroundStyle(AppTheme.text)
                Spacer()
                Text(String(format: "白 %.1f%%", (1 - winrateBlack) * 100))
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(AppTheme.secondaryText)
            }
        }
    }
}

private struct ScoreLeadCompactBar: View {
    let leadBlack: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("目差")
                .font(.footnote)
                .foregroundStyle(AppTheme.secondaryText)

            GeometryReader { g in
                let validLead = leadBlack.isFinite ? leadBlack : 0
                let clamped = max(-30, min(30, validLead))
                let center = g.size.width / 2
                let barWidth = min(center, abs(clamped) / 30 * center)

                ZStack(alignment: .center) {
                    RoundedRectangle(cornerRadius: 4, style: .continuous)
                        .fill(AppTheme.controlTrack)
                        .frame(height: AppTheme.Metrics.chartHeight)

                    if barWidth > 1.5 {
                        RoundedRectangle(cornerRadius: 4, style: .continuous)
                            .fill(clamped >= 0 ? Color.black.opacity(0.82) : Color.white.opacity(0.85))
                            .frame(width: barWidth, height: AppTheme.Metrics.chartHeight)
                            .offset(x: clamped >= 0 ? -(barWidth / 2) : barWidth / 2)
                    }

                    Rectangle()
                        .fill(AppTheme.hairline)
                        .frame(width: 1, height: AppTheme.Metrics.chartHeight)
                }
            }
            .frame(height: AppTheme.Metrics.chartHeight)

            HStack {
                Text(String(format: "黑 %+.1f", leadBlack.isFinite ? leadBlack : 0))
                    .font(.caption.monospacedDigit().weight(.semibold))
                    .foregroundStyle(leadBlack > 0.5 ? AppTheme.text : AppTheme.secondaryText)
                Spacer()
                Text("均势")
                    .font(.caption)
                    .foregroundStyle(AppTheme.tertiaryText)
                Spacer()
                Text(String(format: "白 %+.1f", leadBlack.isFinite ? -leadBlack : 0))
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(leadBlack < -0.5 ? AppTheme.text : AppTheme.secondaryText)
            }
        }
    }
}
