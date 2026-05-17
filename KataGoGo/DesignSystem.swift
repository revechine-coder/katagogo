import SwiftUI

enum AppTheme {
    static let appBackground = Color(red: 0.94, green: 0.95, blue: 0.94)
    static let surface = Color(red: 0.98, green: 0.985, blue: 0.98)
    static let elevatedSurface = Color.white.opacity(0.82)
    static let hairline = Color.black.opacity(0.08)
    static let text = Color(red: 0.12, green: 0.14, blue: 0.14)
    static let secondaryText = Color(red: 0.43, green: 0.47, blue: 0.46)
    static let teal = Color(red: 0.12, green: 0.48, blue: 0.48)
    static let tealSoft = Color(red: 0.78, green: 0.88, blue: 0.86)
    static let woodLight = Color(red: 0.86, green: 0.67, blue: 0.42)
    static let woodDark = Color(red: 0.52, green: 0.34, blue: 0.18)
    
    static let panelRadius: CGFloat = 8
    static let controlRadius: CGFloat = 7
}

struct PanelStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .background(AppTheme.elevatedSurface)
            .clipShape(RoundedRectangle(cornerRadius: AppTheme.panelRadius, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: AppTheme.panelRadius, style: .continuous)
                    .stroke(AppTheme.hairline, lineWidth: 1)
            )
            .shadow(color: .black.opacity(0.04), radius: 10, x: 0, y: 6)
    }
}

extension View {
    func panelStyle() -> some View {
        modifier(PanelStyle())
    }
}

struct MetricRow: View {
    let title: String
    let value: String
    var accent: Color = AppTheme.text
    
    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(title)
                .font(.caption)
                .foregroundStyle(AppTheme.secondaryText)
            Spacer()
            Text(value)
                .font(.system(.subheadline, design: .rounded, weight: .semibold))
                .monospacedDigit()
                .foregroundStyle(accent)
        }
    }
}
