import SwiftUI
struct GameToolbar: View {
    @ObservedObject var viewModel: GameViewModel
    @State private var showsEngineSettings = false
    @State private var showsGameLibrary = false

    var body: some View {
        GeometryReader { proxy in
            let isCompact = proxy.size.width < 1080

            HStack(spacing: isCompact ? 8 : 13) {
                AppMark()

                Divider().frame(height: 26)

                toolbarButton(
                    title: "开始",
                    systemImage: "play.fill",
                    isDisabled: viewModel.phase == .connecting || viewModel.phase == .waiting || viewModel.phase == .playing
                ) {
                    viewModel.startOrResumeGame()
                }

                toolbarButton(title: "暂停", systemImage: "pause.fill", isDisabled: viewModel.phase != .playing) {
                    viewModel.pauseGame()
                }

                toolbarButton(
                    title: "新局",
                    systemImage: "doc.badge.plus",
                    isDisabled: viewModel.phase == .connecting || viewModel.phase == .waiting
                ) {
                    viewModel.newGame()
                }

                toolbarButton(
                    title: "保存",
                    systemImage: "square.and.arrow.down",
                    isDisabled: viewModel.moveCount == 0
                ) {
                    _ = viewModel.saveGame()
                }

                toolbarButton(
                    title: "棋谱库",
                    systemImage: "folder",
                    isDisabled: false
                ) {
                    showsGameLibrary = true
                }

                Divider().frame(height: 26)

                toolbarButton(title: "悔棋", systemImage: "arrow.uturn.backward", isDisabled: viewModel.phase != .playing) {
                    viewModel.undo()
                }
                .keyboardShortcut("z", modifiers: .command)

                toolbarButton(
                    title: "估目",
                    systemImage: "sum",
                    isDisabled: (viewModel.phase != .playing && viewModel.phase != .paused) || viewModel.isScoringFinalResult
                ) {
                    viewModel.countFinalScore()
                }

                toolbarButton(
                    title: "重置",
                    systemImage: "arrow.clockwise",
                    isDisabled: viewModel.phase == .idle || viewModel.phase == .connecting || viewModel.phase == .waiting
                ) {
                    viewModel.resetGame()
                }

                Divider().frame(height: 26)
                HumanColorPicker(viewModel: viewModel, compact: isCompact)
                Divider().frame(height: 26)
                ModelMenu(viewModel: viewModel, compact: isCompact)
                HandicapPicker(viewModel: viewModel, compact: isCompact)
                Divider().frame(height: 26)
                SuggestionToggle(viewModel: viewModel)
                Spacer(minLength: 8)
                EngineStatusView(
                    phase: viewModel.phase,
                    finalScoreText: viewModel.finalScoreText,
                    isScoringFinalResult: viewModel.isScoringFinalResult,
                    compact: isCompact
                )
                if !isCompact {
                    Divider().frame(height: 28)
                    IconToolbarButton(title: "关于", systemImage: "info.circle")
                }
            }
            .padding(.horizontal, isCompact ? AppTheme.Spacing.md : AppTheme.Spacing.lg)
            .padding(.vertical, 10)
            .frame(width: proxy.size.width, height: AppTheme.toolbarHeight)
        }
        .buttonStyle(.plain)
        .controlSize(.regular)
        .font(.body.weight(.medium))
        .frame(height: AppTheme.toolbarHeight)
        .background(AppTheme.surface)
        .overlay(Rectangle().fill(AppTheme.hairline).frame(height: 1), alignment: .bottom)
        .sheet(isPresented: $showsEngineSettings) {
            EngineSettingsView(viewModel: viewModel)
        }
        .sheet(isPresented: $showsGameLibrary) {
            GameLibraryView(viewModel: viewModel)
        }
    }

    @ViewBuilder
    private func toolbarButton(title: String, systemImage: String, isDisabled: Bool, action: @escaping () -> Void) -> some View {
        ToolbarIconButton(
            title: title,
            systemImage: systemImage,
            isDisabled: isDisabled,
            action: action
        )
    }
}

private struct ToolbarIconButton: View {
    let title: String
    let systemImage: String
    let isDisabled: Bool
    var tint: Color = AppTheme.text
    var isActive: Bool = false
    let action: () -> Void

    @State private var isHovered = false

    var body: some View {
        ZStack(alignment: .bottom) {
            Button(action: action) {
                Image(systemName: systemImage)
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(isDisabled ? AppTheme.tertiaryText : tint)
                    .frame(width: AppTheme.Metrics.controlHeight, height: AppTheme.Metrics.controlHeight)
                    .background(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .fill(buttonBackground)
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .stroke(isActive ? AppTheme.teal.opacity(0.34) : Color.clear, lineWidth: 1)
                    )
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .disabled(isDisabled)
            .accessibilityLabel(title)

            if isHovered {
                TooltipLabel(title: title)
                    .offset(y: 34)
                    .transition(.opacity.combined(with: .scale(scale: 0.96)))
                    .zIndex(5)
            }
        }
        .frame(width: AppTheme.Metrics.controlHeight, height: AppTheme.Metrics.controlHeight)
        .onHover { hovering in
            withAnimation(.easeOut(duration: 0.12)) {
                isHovered = hovering
            }
        }
        .zIndex(isHovered ? 10 : 0)
    }

    private var buttonBackground: Color {
        if isActive {
            return AppTheme.tealSoft.opacity(isHovered ? 0.72 : 0.48)
        }
        return isHovered && !isDisabled ? AppTheme.controlTrack : Color.clear
    }
}

private struct TooltipLabel: View {
    let title: String

    var body: some View {
        Text(title)
            .font(.system(size: 12, weight: .medium))
            .foregroundStyle(.white)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(
                RoundedRectangle(cornerRadius: 5, style: .continuous)
                    .fill(Color.black.opacity(0.78))
            )
            .fixedSize()
            .allowsHitTesting(false)
    }
}

private struct AppMark: View {
    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 7, style: .continuous)
                .fill(
                    LinearGradient(
                        colors: [
                            AppTheme.woodLight.opacity(0.98),
                            AppTheme.woodDark.opacity(0.88)
                        ],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 7, style: .continuous)
                        .stroke(Color.black.opacity(0.16), lineWidth: 1)
                )
            VStack(spacing: 3) {
                ForEach(0..<3, id: \.self) { _ in
                    Rectangle()
                        .fill(Color.black.opacity(0.18))
                        .frame(width: 20, height: 0.7)
                }
            }
            .rotationEffect(.degrees(-8))
            Circle()
                .fill(Color.black.opacity(0.88))
                .frame(width: 11, height: 11)
                .offset(x: -6, y: -6)
            Circle()
                .fill(Color.white.opacity(0.94))
                .overlay(Circle().stroke(Color.black.opacity(0.16), lineWidth: 0.7))
                .frame(width: 11, height: 11)
                .offset(x: 6, y: 6)
            Image(systemName: "sparkle")
                .font(.system(size: 9, weight: .bold))
                .foregroundStyle(AppTheme.tealSoft)
                .offset(x: 6, y: -6)
        }
        .frame(width: 30, height: 30)
        .shadow(color: Color.black.opacity(0.12), radius: 3, x: 0, y: 1)
    }
}

private struct HumanColorPicker: View {
    @ObservedObject var viewModel: GameViewModel
    var compact: Bool = false

    var body: some View {
        HStack(spacing: 8) {
            if !compact {
                Text("执子")
                    .font(.footnote)
                    .foregroundStyle(AppTheme.secondaryText)
            }
            HumanColorSegmentedControl(
                selection: viewModel.humanColor,
                isEnabled: viewModel.canChooseHumanColor
            ) { color in
                viewModel.setHumanColor(color)
            }
            .frame(width: compact ? 98 : 112, height: AppTheme.Metrics.controlHeight)
            .opacity(viewModel.canChooseHumanColor ? 1 : 0.48)
        }
    }
}

private struct HumanColorSegmentedControl: View {
    let selection: String
    let isEnabled: Bool
    let onSelect: (String) -> Void

    var body: some View {
        HStack(spacing: 2) {
            segment(title: "执黑", color: "b")
            segment(title: "执白", color: "w")
        }
        .padding(2)
        .background(AppTheme.controlTrack)
        .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous)
                .stroke(AppTheme.hairline, lineWidth: 1)
        )
    }

    private func segment(title: String, color: String) -> some View {
        let isSelected = selection == color
        return Button {
            guard isEnabled else { return }
            onSelect(color)
        } label: {
            Text(title)
                .font(.footnote.weight(.semibold))
                .lineLimit(1)
                .foregroundStyle(foregroundColor(for: color, isSelected: isSelected))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(backgroundColor(for: color, isSelected: isSelected))
                .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius - 2, style: .continuous))
        }
        .buttonStyle(.plain)
        .disabled(!isEnabled)
        .accessibilityLabel(title)
    }

    private func foregroundColor(for color: String, isSelected: Bool) -> Color {
        if isSelected && color == "b" { return .white }
        return AppTheme.text
    }

    private func backgroundColor(for color: String, isSelected: Bool) -> Color {
        guard isSelected else { return .clear }
        return color == "b" ? Color.black.opacity(0.86) : Color.white.opacity(0.92)
    }
}

private struct ModelMenu: View {
    @ObservedObject var viewModel: GameViewModel
    var compact: Bool = false

    var body: some View {
        Button {} label: {
            if compact {
                Image(systemName: "gearshape")
            } else {
                Label("配置引擎", systemImage: "gearshape")
            }
        }
        .buttonStyle(.plain)
        .disabled(true)
        .help("封装版已固定使用内置 KataGo 引擎")
        .foregroundStyle(AppTheme.tertiaryText)
        .frame(width: compact ? 40 : 88, height: AppTheme.Metrics.controlHeight)
    }
}

private struct IconToolbarButton: View {
    let title: String
    let systemImage: String

    var body: some View {
        Button {} label: {
            Image(systemName: systemImage)
                .font(.system(size: 16, weight: .medium))
            .foregroundStyle(AppTheme.text)
                .frame(width: 38, height: 32)
        }
        .buttonStyle(.plain)
        .help(title)
    }
}

private struct SuggestionToggle: View {
    @ObservedObject var viewModel: GameViewModel

    var body: some View {
        ToolbarIconButton(
            title: viewModel.isShowingSuggestions ? "隐藏建议" : "显示建议",
            systemImage: viewModel.isShowingSuggestions ? "sparkle" : "lightbulb",
            isDisabled: false,
            tint: viewModel.isShowingSuggestions ? AppTheme.tealDark : AppTheme.secondaryText,
            isActive: viewModel.isShowingSuggestions,
            action: { viewModel.setShowingSuggestions(!viewModel.isShowingSuggestions) }
        )
    }
}

private struct EngineStatusView: View {
    let phase: GamePhase
    let finalScoreText: String?
    let isScoringFinalResult: Bool
    var compact: Bool = false
    
    var body: some View {
        HStack(spacing: 8) {
            if !compact && !showsScoreResult {
                Text("引擎状态")
                    .foregroundStyle(AppTheme.secondaryText)
            }
            if phase == .waiting || phase == .connecting {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 10, height: 10)
            } else {
                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)
                    .frame(width: 10, height: 10)
            }
            Text(statusText)
                .foregroundStyle(AppTheme.text)
                .lineLimit(1)
                .minimumScaleFactor(0.86)
        }
        .font(.footnote)
        .padding(.horizontal, compact ? 6 : 10)
        .padding(.vertical, 5)
        .frame(width: statusWidth, height: AppTheme.Metrics.controlHeight, alignment: .center)
    }

    private var showsScoreResult: Bool {
        finalScoreText != nil || isScoringFinalResult
    }

    private var statusWidth: CGFloat {
        if showsScoreResult {
            return compact ? 174 : 236
        }
        return compact ? 116 : 188
    }
    
    private var statusText: String {
        if isScoringFinalResult {
            return "终局计算"
        }

        if let finalScoreText {
            return finalScoreText
        }

        return switch phase {
        case .idle: "未开始"
        case .connecting: "准备中"
        case .playing: "引擎就绪"
        case .paused: "已暂停"
        case .waiting: "AI 思考"
        case .finished: "对局结束"
        }
    }
    
    private var statusColor: Color {
        switch phase {
        case .playing: AppTheme.teal
        case .paused: AppTheme.secondaryText
        case .finished: AppTheme.warning
        default: AppTheme.secondaryText
        }
    }
}
