import SwiftUI
struct LevelPicker: View {
    let viewModel: GameViewModel
    let levels = [(0,"9级"),(1,"7k~8k"),(2,"4k~6k"),(3,"1k~3k"),(4,"1d~3d"),(5,"4d~6d"),(6,"7d+")]
    @State private var selectedLevel: Int = 4
    var body: some View {
        Picker("AI 棋力", selection: $selectedLevel) {
            ForEach(levels, id: \.0) { i, l in Text(l).tag(i) }
        }
        .pickerStyle(.menu)
        .frame(width: 132)
        .font(.system(.subheadline, design: .rounded, weight: .medium))
        .onChange(of: selectedLevel) { _, n in viewModel.setLevel(n) }
    }
}
