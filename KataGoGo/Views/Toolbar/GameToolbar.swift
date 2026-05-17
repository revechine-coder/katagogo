import SwiftUI
struct GameToolbar: View {
    let viewModel: GameViewModel
    var body: some View {
        HStack(spacing: 14) {
            HStack(spacing: 8) {
                Image(systemName: "circle.grid.3x3.fill")
                    .font(.system(size: 17, weight: .semibold))
                    .foregroundStyle(AppTheme.teal)
                Text("KataGoGo")
                    .font(.system(.headline, design: .rounded, weight: .semibold))
                    .foregroundStyle(AppTheme.text)
            }
            
            Divider().frame(height: 24)
            
            Button { viewModel.resetGame() } label: {
                Label("清盘", systemImage: "arrow.counterclockwise")
            }
            .disabled(viewModel.phase != .playing)
            
            Button { viewModel.undo() } label: {
                Label("悔棋", systemImage: "arrow.uturn.backward")
            }
            .disabled(viewModel.phase != .playing)
            .keyboardShortcut("z", modifiers: .command)
            
            Divider().frame(height: 24)
            LevelPicker(viewModel: viewModel)
            Spacer()
            EngineStatusView(phase: viewModel.phase)
        }
        .buttonStyle(.borderless)
        .labelStyle(.titleAndIcon)
        .font(.system(.subheadline, design: .rounded, weight: .medium))
        .padding(.horizontal, 18)
        .padding(.vertical, 11)
        .frame(height: 50)
        .background(AppTheme.surface)
        .overlay(Rectangle().fill(AppTheme.hairline).frame(height: 1), alignment: .bottom)
    }
}

private struct EngineStatusView: View {
    let phase: GamePhase
    
    var body: some View {
        HStack(spacing: 8) {
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
                .font(.caption)
                .foregroundStyle(AppTheme.secondaryText)
                .lineLimit(1)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .frame(width: 122, height: 28, alignment: .leading)
        .background(AppTheme.tealSoft.opacity(0.45))
        .clipShape(RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
    }
    
    private var statusText: String {
        switch phase {
        case .idle: "未启动"
        case .connecting: "启动引擎..."
        case .playing: "引擎就绪"
        case .waiting: "AI 思考中..."
        case .finished: "对局结束"
        }
    }
    
    private var statusColor: Color {
        switch phase {
        case .playing: AppTheme.teal
        case .finished: .orange
        default: AppTheme.secondaryText
        }
    }
}
