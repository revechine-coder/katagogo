import SwiftUI
struct ScoreLeadView: View {
    let leadBlack: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("目差")
                    .font(.footnote)
                    .foregroundStyle(AppTheme.secondaryText)
                Spacer()
                Text(leadText)
                    .font(.title3.weight(.semibold))
                    .monospacedDigit()
                    .foregroundStyle(AppTheme.text)
            }
            GeometryReader { g in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4).fill(AppTheme.controlTrack).frame(height: AppTheme.Metrics.chartHeight)
                    let clamped = max(-30, min(30, leadBlack))
                    let center = g.size.width / 2
                    let width = min(center, abs(clamped) / 30 * center)
                    let startX = clamped >= 0 ? center - width : center
                    RoundedRectangle(cornerRadius: 5)
                        .fill(clamped >= 0 ? Color.black.opacity(0.82) : Color.white.opacity(0.85))
                        .frame(width: width, height: AppTheme.Metrics.chartHeight)
                        .position(x: startX + width / 2, y: AppTheme.Metrics.chartHeight / 2)
                    Rectangle().fill(Color.white.opacity(0.60)).frame(width: 1, height: AppTheme.Metrics.chartHeight).position(x: g.size.width/2, y: AppTheme.Metrics.chartHeight / 2)
                    Text("黑棋").font(.caption).foregroundColor(.white).position(x: 18, y: AppTheme.Metrics.chartHeight / 2)
                    Text("白棋").font(.caption).foregroundColor(.black.opacity(0.72)).position(x: g.size.width-24, y: AppTheme.Metrics.chartHeight / 2)
                }
            }.frame(height: AppTheme.Metrics.chartHeight)
            HStack {
                Text("黑棋")
                Spacer()
                Text(centerText)
                Spacer()
                Text("白棋")
            }
            .font(.caption)
            .foregroundStyle(AppTheme.secondaryText)
        }
        .padding(AppTheme.Metrics.panelPadding)
        .panelStyle()
    }

    private var leadText: String {
        guard leadBlack.isFinite else { return "暂无目差信息" }
        return String(format: "黑棋 %+.1f / 白棋 %+.1f", leadBlack, -leadBlack)
    }

    private var centerText: String {
        guard leadBlack.isFinite else { return "无数据" }
        return String(format: "黑棋 %+.1f 目", leadBlack)
    }
}
