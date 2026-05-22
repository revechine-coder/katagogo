import SwiftUI

struct ReviewPanel: View {
    @ObservedObject var viewModel: GameViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            PanelHeader(
                title: "全盘体检",
                systemImage: "stethoscope",
                trailingSystemImage: nil
            )

            if viewModel.isScanningReview {
                scanningView
            } else if let report = viewModel.autoReviewReport {
                reportContent(report)
            } else {
                idleView
            }
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }

    // MARK: - Idle

    private var idleView: some View {
        VStack(spacing: 12) {
            Text("让 KataGo 自动扫描整局棋谱\n标记所有恶手与缓着")
                .font(.caption)
                .foregroundStyle(AppTheme.secondaryText)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)

            scanButton
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Scanning

    private var scanningView: some View {
        VStack(spacing: 14) {
            ProgressView()
                .scaleEffect(1.2)
                .controlSize(.small)

            Text("正在分析每一手...")
                .font(.caption.weight(.medium))
                .foregroundStyle(AppTheme.secondaryText)

            Text("这可能需要数十秒，具体取决于棋局长度")
                .font(.caption2)
                .foregroundStyle(AppTheme.tertiaryText)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 20)
    }

    // MARK: - Report Content

    private func reportContent(_ report: AutoReviewReport) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            summarySection(report)
            moveListSection(report)
        }
    }

    // MARK: - Summary

    private func summarySection(_ report: AutoReviewReport) -> some View {
        VStack(spacing: 8) {
            HStack(spacing: 0) {
                summaryBlock(
                    count: report.badMoveCount,
                    label: "恶手",
                    color: AppTheme.danger
                )
                summaryBlock(
                    count: report.slackMoveCount,
                    label: "缓着",
                    color: AppTheme.warning
                )
                summaryBlock(
                    count: report.goodMoveCount,
                    label: "好手",
                    color: AppTheme.teal
                )
            }

            scanButton
                .controlSize(.small)
        }
    }

    private func summaryBlock(count: Int, label: String, color: Color) -> some View {
        VStack(spacing: 2) {
            Text("\(count)")
                .font(.title2.weight(.bold).monospacedDigit())
                .foregroundStyle(color)
            Text(label)
                .font(.caption2.weight(.medium))
                .foregroundStyle(AppTheme.secondaryText)
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Scan Button

    private var scanButton: some View {
        Button {
            viewModel.runAutoReview()
        } label: {
            HStack(spacing: 5) {
                Image(systemName: viewModel.isScanningReview ? "hourglass" : "magnifyingglass.circle.fill")
                    .font(.system(size: 12, weight: .semibold))
                Text(viewModel.autoReviewReport != nil ? "重新体检" : "开始体检")
                    .font(.caption.weight(.semibold))
            }
            .foregroundStyle(.white)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
            .background(viewModel.isScanningReview ? AppTheme.tertiaryText : AppTheme.teal)
            .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
        }
        .buttonStyle(.plain)
        .disabled(viewModel.isScanningReview || viewModel.moveCount == 0)
    }

    // MARK: - Move List

    private func moveListSection(_ report: AutoReviewReport) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            if report.hasIssues {
                Text("失误详情")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(AppTheme.text)
                    .padding(.bottom, 2)
            }

            let problemMoves = report.moves.filter { $0.quality != .good }

            if problemMoves.isEmpty {
                Text("未发现明显失误，表现完美！")
                    .font(.caption)
                    .foregroundStyle(AppTheme.teal)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 8)
            } else {
                ScrollView {
                    LazyVStack(spacing: 4) {
                        ForEach(problemMoves) { item in
                            moveRow(item)
                        }
                    }
                }
                .frame(maxHeight: 280)
            }
        }
    }

    private func moveRow(_ item: ReviewedMoveItem) -> some View {
        Button {
            viewModel.didSelectReviewItem(item)
        } label: {
            HStack(spacing: 6) {
                // Move number
                Text("#\(item.moveNumber)")
                    .font(.caption.monospacedDigit().weight(.bold))
                    .foregroundStyle(AppTheme.text)
                    .frame(width: 28, alignment: .leading)

                // Color dot
                Circle()
                    .fill(item.color == "b" ? Color.black : Color.white)
                    .overlay(Circle().stroke(AppTheme.hairline, lineWidth: 1))
                    .frame(width: 10, height: 10)

                // Vertex
                Text(item.vertex)
                    .font(.caption.monospaced().weight(.medium))
                    .foregroundStyle(AppTheme.text)
                    .frame(width: 36, alignment: .leading)

                Spacer(minLength: 2)

                // Quality badge
                qualityBadge(item.quality)

                Spacer(minLength: 2)

                // Winrate drop
                if item.winrateDrop > 0.005 {
                    Text(String(format: "-%.1f%%", item.winrateDrop * 100))
                        .font(.caption2.monospacedDigit().weight(.medium))
                        .foregroundStyle(AppTheme.danger)
                        .frame(width: 44, alignment: .trailing)
                }

                // Score drop
                if item.scoreDrop > 0.1 {
                    Text(String(format: "-%.1f目", item.scoreDrop))
                        .font(.caption2.monospacedDigit().weight(.medium))
                        .foregroundStyle(AppTheme.warning)
                        .frame(width: 44, alignment: .trailing)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 5)
            .background(
                RoundedRectangle(cornerRadius: 5, style: .continuous)
                    .fill(Color.primary.opacity(0.04))
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help("点击跳转到第 \(item.moveNumber) 手前的盘面，并显示 AI 推荐着点")
    }

    private func qualityBadge(_ quality: ReviewedMoveQuality) -> some View {
        let (label, color): (String, Color) = switch quality {
        case .badMove: ("恶手", AppTheme.danger)
        case .slackMove: ("缓着", AppTheme.warning)
        case .good: ("", .clear)
        }

        return Text(label)
            .font(.caption2.weight(.bold))
            .foregroundStyle(color)
            .padding(.horizontal, 4)
            .padding(.vertical, 2)
            .background(color.opacity(0.12))
            .clipShape(RoundedRectangle(cornerRadius: 3, style: .continuous))
    }
}
