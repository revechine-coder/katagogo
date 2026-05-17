import SwiftUI
struct ScoreLeadView: View {
    let lead: Double
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("目差形势")
                    .font(.caption)
                    .foregroundStyle(AppTheme.secondaryText)
                Spacer()
                Text(lt)
                    .font(.system(.title3, design: .rounded, weight: .semibold))
                    .monospacedDigit()
                    .foregroundStyle(AppTheme.text)
            }
            GeometryReader { g in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 5).fill(Color.white).frame(height: 24)
                    let clamped = max(-20, min(20, lead)), fw = ((clamped+20)/40)*g.size.width
                    RoundedRectangle(cornerRadius: 5)
                        .fill(LinearGradient(colors: [.black, AppTheme.teal, .white], startPoint: .leading, endPoint: .trailing))
                        .frame(width: max(0, fw), height: 24)
                    Rectangle().fill(AppTheme.teal.opacity(0.55)).frame(width: 1, height: 24).position(x: g.size.width/2, y: 12)
                    Text("黑优").font(.caption2).foregroundColor(.white).position(x: 18, y: 12)
                    Text("白优").font(.caption2).foregroundColor(.black.opacity(0.72)).position(x: g.size.width-18, y: 12)
                }
            }.frame(height: 20)
        }
        .padding(14)
        .panelStyle()
    }
    private var lt: String {
        let a = abs(lead)
        if a < 0.1 { return "双方均势" }
        return lead > 0 ? "黑+\(String(format: "%.1f", a))目" : "白+\(String(format: "%.1f", a))目"
    }
}
