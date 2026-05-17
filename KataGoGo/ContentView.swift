import SwiftUI

struct ContentView: View {
    @State private var viewModel = GameViewModel()
    @State private var showErrorAlert = false
    
    var body: some View {
        VStack(spacing: 0) {
            GameToolbar(viewModel: viewModel)
            mainLayout
        }
        .background(AppTheme.appBackground)
        .onAppear { viewModel.startGame() }
        .onChange(of: viewModel.errorMessage) { _, newValue in
            showErrorAlert = newValue != nil
        }
        .alert("错误", isPresented: $showErrorAlert) {
            Button("确定", role: .cancel) { viewModel.errorMessage = nil }
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
        .frame(minWidth: 980, minHeight: 640)
    }
    
    private var mainLayout: some View {
        HStack(alignment: .top, spacing: 18) {
            leftColumn
            boardStage
            AnalysisPanel(viewModel: viewModel)
        }
        .padding(18)
    }
    
    private var leftColumn: some View {
        VStack(spacing: 14) {
            MiniMapView(viewModel: viewModel)
            MoveTimelineView(viewModel: viewModel)
            Spacer(minLength: 0)
        }
        .frame(width: 224)
    }
    
    private var boardStage: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(Color.white.opacity(0.35))
                .shadow(color: .black.opacity(0.06), radius: 18, x: 0, y: 12)
            
            BoardCanvas(viewModel: viewModel)
                .padding(22)
        }
        .frame(minWidth: 460, idealWidth: 680, maxWidth: .infinity,
               minHeight: 460, idealHeight: 680, maxHeight: .infinity)
    }
}
