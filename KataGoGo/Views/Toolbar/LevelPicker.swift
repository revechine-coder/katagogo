import SwiftUI
struct HandicapPicker: View {
    @ObservedObject var viewModel: GameViewModel
    private let options = [0, 2, 3, 4, 5, 6, 7, 8, 9]
    var compact: Bool = false

    var body: some View {
        Menu {
            ForEach(options, id: \.self) { count in
                Button {
                    viewModel.setHandicapCount(count)
                } label: {
                    if viewModel.handicapCount == count {
                        Label(label(for: count), systemImage: "checkmark")
                    } else {
                        Text(label(for: count))
                    }
                }
                .disabled(!viewModel.canChooseHandicap)
            }
        } label: {
            if compact {
                Text(label(for: viewModel.handicapCount))
            } else {
                Label(label(for: viewModel.handicapCount), systemImage: "circle.grid.cross")
            }
        }
        .help(viewModel.canChooseHandicap ? "选择让子数" : "开局后不能更改让子")
        .foregroundStyle(AppTheme.text)
        .frame(minWidth: compact ? 48 : 78, idealWidth: compact ? 56 : 94, minHeight: AppTheme.Metrics.controlHeight)
        .font(.body.weight(.medium))
    }

    private func label(for count: Int) -> String {
        count == 0 ? "不让子" : "让 \(count) 子"
    }
}
