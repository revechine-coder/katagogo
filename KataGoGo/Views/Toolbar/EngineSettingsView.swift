import SwiftUI
import UniformTypeIdentifiers

struct EngineSettingsView: View {
    @Environment(\.dismiss) private var dismiss

    let viewModel: GameViewModel

    @ObservedObject private var appState = AppState.shared
    @State private var isShowingFileImporter = false
    @State private var filePickerTarget: EngineFileTarget?
    @State private var originalBinaryPath: String = AppState.shared.kataGoBinaryPath
    @State private var originalConfigPath: String = AppState.shared.kataGoConfigPath
    @State private var originalModelPath: String = AppState.shared.kataGoModelPath

    var body: some View {
        VStack(alignment: .leading, spacing: AppTheme.Spacing.lg) {
            HStack {
                PanelHeader(title: "KataGo 引擎", systemImage: "cpu", trailingSystemImage: nil)
                Spacer()
                Button {
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 12, weight: .bold))
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(.plain)
                .help("关闭")
            }

            VStack(spacing: AppTheme.Spacing.md) {
                enginePathRow(
                    title: "程序",
                    path: appState.kataGoBinaryPath,
                    target: .binary,
                    systemImage: "terminal"
                )
                enginePathRow(
                    title: "配置",
                    path: appState.kataGoConfigPath,
                    target: .config,
                    systemImage: "doc.text"
                )
                enginePathRow(
                    title: "模型",
                    path: appState.kataGoModelPath,
                    target: .model,
                    systemImage: "brain"
                )
            }

            if let configurationError = appState.configurationError {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundStyle(AppTheme.warning)
                    Text(configurationError)
                        .foregroundStyle(AppTheme.text)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .font(.footnote)
                .padding(AppTheme.Spacing.md)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(AppTheme.warning.opacity(0.11))
                .clipShape(RoundedRectangle(cornerRadius: AppTheme.panelRadius, style: .continuous))
            } else {
                InlineStatusPill(title: "配置可用", systemImage: "checkmark.circle.fill", tint: AppTheme.teal)
            }

            HStack {
                Button("恢复默认") {
                    appState.resetEnginePathsToDefaults()
                    viewModel.modelName = appState.modelDisplayName
                }
                .buttonStyle(.bordered)

                Spacer()

                Button("完成") {
                    viewModel.modelName = appState.modelDisplayName
                    let pathsChanged = appState.kataGoBinaryPath != originalBinaryPath
                        || appState.kataGoConfigPath != originalConfigPath
                        || appState.kataGoModelPath != originalModelPath
                    if pathsChanged && viewModel.phase != .idle {
                        viewModel.endGame()
                    }
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(AppTheme.Spacing.xl)
        .frame(width: 620)
        .fileImporter(
            isPresented: $isShowingFileImporter,
            allowedContentTypes: [.item],
            allowsMultipleSelection: false
        ) { result in
            guard let target = filePickerTarget else { return }
            filePickerTarget = nil
            guard case .success(let urls) = result, let url = urls.first else { return }
            setPath(url.path, for: target)
        }
    }

    private func enginePathRow(
        title: String,
        path: String,
        target: EngineFileTarget,
        systemImage: String
    ) -> some View {
        HStack(spacing: AppTheme.Spacing.md) {
            Image(systemName: systemImage)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(AppTheme.tealDark)
                .frame(width: 22)

            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.footnote)
                    .foregroundStyle(AppTheme.secondaryText)
                Text(path)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(AppTheme.text)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }

            Spacer(minLength: AppTheme.Spacing.md)

            if appState.isUsingBundledEngine {
                InlineStatusPill(title: "已封装", systemImage: "shippingbox.fill", tint: AppTheme.teal)
            } else {
                Button("选择") {
                    filePickerTarget = target
                    isShowingFileImporter = true
                }
                .buttonStyle(.bordered)
            }
        }
        .padding(AppTheme.Spacing.md)
        .panelStyle(.bordered)
    }

    private func setPath(_ path: String, for target: EngineFileTarget) {
        switch target {
        case .binary:
            appState.kataGoBinaryPath = path
        case .config:
            appState.kataGoConfigPath = path
        case .model:
            appState.kataGoModelPath = path
            viewModel.modelName = appState.modelDisplayName
        }
    }
}

private enum EngineFileTarget: Identifiable {
    case binary
    case config
    case model

    var id: String {
        switch self {
        case .binary: "binary"
        case .config: "config"
        case .model: "model"
        }
    }
}
