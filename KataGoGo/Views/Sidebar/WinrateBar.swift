import SwiftUI
struct WinrateBar: View {
    let winrateBlack: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("胜率")
                    .font(.footnote)
                    .foregroundStyle(AppTheme.secondaryText)
                Spacer()
                Text(winrateText)
                    .font(.title3.weight(.semibold))
                    .monospacedDigit()
                    .foregroundStyle(AppTheme.text)
            }
            GeometryReader { g in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4).fill(AppTheme.controlTrack).frame(height: AppTheme.Metrics.chartHeight)
                    HStack(spacing: 0) {
                        let clamped = max(0, min(1, winrateBlack))
                        let blackWidth = g.size.width * clamped
                        RoundedRectangle(cornerRadius: 4)
                            .fill(LinearGradient(colors: [Color(red: 0.12, green: 0.13, blue: 0.14), .black], startPoint: .leading, endPoint: .trailing))
                            .frame(width: blackWidth, height: AppTheme.Metrics.chartHeight)
                        RoundedRectangle(cornerRadius: 4)
                            .fill(Color.white.opacity(0.85))
                            .frame(width: g.size.width - blackWidth, height: AppTheme.Metrics.chartHeight)
                    }
                    Rectangle().fill(Color.white.opacity(0.60)).frame(width: 1, height: AppTheme.Metrics.chartHeight).position(x: g.size.width/2, y: AppTheme.Metrics.chartHeight / 2)
                    Text(String(format: "%.1f%%", winrateBlack * 100))
                        .font(.caption.monospacedDigit().weight(.semibold))
                        .foregroundColor(.white)
                        .position(x: 28, y: AppTheme.Metrics.chartHeight / 2)
                    Text(String(format: "%.1f%%", (1 - winrateBlack) * 100))
                        .font(.caption.monospacedDigit())
                        .foregroundColor(.black.opacity(0.62))
                        .position(x: g.size.width - 28, y: AppTheme.Metrics.chartHeight / 2)
                }
            }.frame(height: AppTheme.Metrics.chartHeight)
            HStack {
                Text("黑棋")
                Spacer()
                Text("白棋")
            }
            .font(.caption)
            .foregroundStyle(AppTheme.secondaryText)
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }

    private var winrateText: String {
        String(format: "黑棋 %.1f%% / 白棋 %.1f%%", winrateBlack * 100, (1 - winrateBlack) * 100)
    }
}
