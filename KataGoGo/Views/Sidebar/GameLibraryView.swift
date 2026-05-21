import SwiftUI

struct GameLibraryView: View {
    @Environment(\.dismiss) private var dismiss
    let viewModel: GameViewModel

    @State private var games: [GameIndexEntry] = []
    @State private var isLoading = false

    var body: some View {
        VStack(alignment: .leading, spacing: AppTheme.Spacing.lg) {
            HStack {
                PanelHeader(title: "棋谱库", systemImage: "folder", trailingSystemImage: nil)
                Spacer()
                Button { dismiss() } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 12, weight: .bold))
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(.plain)
                .help("关闭")
            }

            if games.isEmpty {
                VStack(spacing: AppTheme.Spacing.md) {
                    Image(systemName: "tray")
                        .font(.system(size: 32))
                        .foregroundStyle(AppTheme.tertiaryText)
                    Text("暂无保存的棋局")
                        .font(.footnote)
                        .foregroundStyle(AppTheme.secondaryText)
                    Text("对局中点击\"保存\"按钮即可保存")
                        .font(.caption)
                        .foregroundStyle(AppTheme.tertiaryText)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView(.vertical) {
                    LazyVStack(spacing: AppTheme.Spacing.sm) {
                        ForEach(games, id: \.id) { game in
                            GameRow(entry: game, isLoading: isLoading) {
                                loadGame(game.id)
                            } onDelete: {
                                deleteGame(game.id)
                            }
                        }
                    }
                }
            }
        }
        .padding(AppTheme.Spacing.xl)
        .frame(width: 460, height: 440)
        .onAppear { games = GameStore.shared.listGames() }
        .onReceive(NotificationCenter.default.publisher(for: NSWindow.willCloseNotification)) { _ in
            games = GameStore.shared.listGames()
        }
    }

    private func loadGame(_ id: String) {
        isLoading = true
        dismiss()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            viewModel.loadGame(id: id)
        }
    }

    private func deleteGame(_ id: String) {
        try? GameStore.shared.delete(id: id)
        games = GameStore.shared.listGames()
    }
}

private struct GameRow: View {
    let entry: GameIndexEntry
    let isLoading: Bool
    let onLoad: () -> Void
    let onDelete: () -> Void

    var body: some View {
        HStack(spacing: AppTheme.Spacing.md) {
            VStack(alignment: .leading, spacing: 3) {
                Text(formattedDate)
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(AppTheme.text)
                HStack(spacing: AppTheme.Spacing.sm) {
                    Text(entry.engineModel)
                        .font(.caption)
                        .foregroundStyle(AppTheme.tertiaryText)
                        .lineLimit(1)
                    if let score = entry.finalScore {
                        Text(score)
                            .font(.caption.monospacedDigit())
                            .foregroundStyle(AppTheme.tealDark)
                    }
                }
            }

            Spacer()

            HStack(spacing: 12) {
                VStack(alignment: .trailing, spacing: 2) {
                    Text("\(entry.moveCount) 手")
                        .font(.footnote.weight(.medium))
                        .monospacedDigit()
                        .foregroundStyle(AppTheme.text)
                    Text(entry.humanColor == "b" ? "执黑" : "执白")
                        .font(.caption)
                        .foregroundStyle(AppTheme.secondaryText)
                }
            }

            Button { onLoad() } label: {
                Image(systemName: "arrow.right.circle.fill")
                    .font(.system(size: 18))
                    .foregroundStyle(AppTheme.teal)
            }
            .buttonStyle(.plain)
            .disabled(isLoading)
            .help("加载棋局")

            Button { onDelete() } label: {
                Image(systemName: "trash")
                    .font(.system(size: 13))
                    .foregroundStyle(AppTheme.tertiaryText)
            }
            .buttonStyle(.plain)
            .help("删除")
        }
        .padding(AppTheme.Spacing.md)
        .panelStyle(.bordered)
    }

    private var formattedDate: String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd HH:mm"
        return formatter.string(from: entry.savedAt)
    }
}
