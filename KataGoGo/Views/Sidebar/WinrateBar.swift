import SwiftUI
struct WinrateBar: View {
    let winrateBlack: Double
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("胜率")
                    .font(.caption)
                    .foregroundStyle(AppTheme.secondaryText)
                Spacer()
                Text(String(format: "%.1f%%", winrateBlack*100))
                    .font(.system(.title3, design: .rounded, weight: .semibold))
                    .monospacedDigit()
                    .foregroundStyle(AppTheme.text)
            }
            GeometryReader { g in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 5).fill(Color.white).frame(height: 24)
                    RoundedRectangle(cornerRadius: 5)
                        .fill(LinearGradient(colors: [.black, Color(red: 0.27, green: 0.30, blue: 0.30)], startPoint: .leading, endPoint: .trailing))
                        .frame(width: max(0, min(g.size.width, g.size.width*winrateBlack)), height: 24)
                    Rectangle().fill(AppTheme.teal.opacity(0.55)).frame(width: 1, height: 24).position(x: g.size.width/2, y: 12)
                    Text("黑").font(.caption2).foregroundColor(.white).position(x: 12, y: 12)
                    Text("白").font(.caption2).foregroundColor(.black.opacity(0.72)).position(x: g.size.width-12, y: 12)
                }
            }.frame(height: 20)
        }
        .padding(14)
        .panelStyle()
    }
}
