import SwiftUI
struct AnalysisPanel: View {
    let viewModel: GameViewModel
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 10) {
                Text("AI 分析")
                    .font(.system(.headline, design: .rounded, weight: .semibold))
                    .foregroundStyle(AppTheme.text)
                HStack(spacing: 10) {
                    Circle()
                        .fill(viewModel.currentPlayer == "b" ? Color.black : Color.white)
                        .frame(width: 22, height: 22)
                        .overlay(Circle().stroke(Color.black.opacity(0.22), lineWidth: 1))
                        .shadow(color: .black.opacity(viewModel.currentPlayer == "b" ? 0.22 : 0.08), radius: 3, x: 0, y: 2)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(viewModel.currentPlayer == "b" ? "黑棋行棋" : "白棋行棋")
                            .font(.system(.title3, design: .rounded, weight: .semibold))
                            .foregroundStyle(AppTheme.text)
                        Text("第 \(viewModel.moveCount) 手")
                            .font(.caption)
                            .foregroundStyle(AppTheme.secondaryText)
                    }
                }
            }
            .padding(14)
            .panelStyle()
            
            WinrateBar(winrateBlack: viewModel.winrateBlack)
            ScoreLeadView(lead: viewModel.lead)
            
            VStack(alignment: .leading, spacing: 10) {
                MetricRow(title: "黑棋提子", value: "\(viewModel.capturesBlack)", accent: .black)
                MetricRow(title: "白棋提子", value: "\(viewModel.capturesWhite)", accent: AppTheme.secondaryText)
                MetricRow(title: "最后落点", value: lastMoveText, accent: AppTheme.teal)
            }
            .padding(14)
            .panelStyle()
            
            Spacer()
        }
        .frame(width: 244)
    }
    
    private var lastMoveText: String {
        guard let lastMove = viewModel.lastMove else { return "-" }
        let columns = Array("ABCDEFGHJKLMNOPQRST")
        guard lastMove.col >= 0, lastMove.col < columns.count else { return "-" }
        return "\(columns[lastMove.col])\(19 - lastMove.row)"
    }
}
