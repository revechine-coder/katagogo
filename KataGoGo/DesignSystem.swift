import SwiftUI

private func dynamicColor(light: NSColor, dark: NSColor) -> Color {
    Color(nsColor: NSColor(name: nil) { appearance in
        switch appearance.name {
        case .darkAqua, .vibrantDark, .accessibilityHighContrastDarkAqua, .accessibilityHighContrastVibrantDark:
            return dark
        default:
            return light
        }
    })
}

enum AppTheme {
    // MARK: - Adaptive background/surface
    static let appBackground = dynamicColor(
        light: NSColor(red: 0.952, green: 0.956, blue: 0.952, alpha: 1),
        dark:  NSColor(red: 0.090, green: 0.098, blue: 0.090, alpha: 1)
    )
    static let surface = dynamicColor(
        light: NSColor(red: 0.986, green: 0.988, blue: 0.986, alpha: 1),
        dark:  NSColor(red: 0.125, green: 0.133, blue: 0.125, alpha: 1)
    )
    static let elevatedSurface = dynamicColor(
        light: NSColor(red: 1.000, green: 1.000, blue: 1.000, alpha: 0.88),
        dark:  NSColor(red: 0.165, green: 0.173, blue: 0.165, alpha: 0.88)
    )
    static let glassSurface = dynamicColor(
        light: NSColor(red: 1.000, green: 1.000, blue: 1.000, alpha: 0.54),
        dark:  NSColor(red: 0.180, green: 0.190, blue: 0.180, alpha: 0.46)
    )
    static let boardSurface = dynamicColor(
        light: NSColor(red: 1.000, green: 1.000, blue: 1.000, alpha: 0.42),
        dark:  NSColor(red: 0.145, green: 0.153, blue: 0.145, alpha: 0.50)
    )

    // MARK: - Adaptive separators
    static let hairline = dynamicColor(
        light: NSColor(red: 0, green: 0, blue: 0, alpha: 0.09),
        dark:  NSColor(red: 1, green: 1, blue: 1, alpha: 0.08)
    )

    // MARK: - Adaptive text
    static let text = dynamicColor(
        light: NSColor(red: 0.12, green: 0.14, blue: 0.14, alpha: 1),
        dark:  NSColor(red: 0.88, green: 0.90, blue: 0.88, alpha: 1)
    )
    static let secondaryText = dynamicColor(
        light: NSColor(red: 0.43, green: 0.47, blue: 0.46, alpha: 1),
        dark:  NSColor(red: 0.55, green: 0.58, blue: 0.56, alpha: 1)
    )
    static let tertiaryText = dynamicColor(
        light: NSColor(red: 0.58, green: 0.61, blue: 0.60, alpha: 1),
        dark:  NSColor(red: 0.40, green: 0.43, blue: 0.41, alpha: 1)
    )

    // MARK: - Adaptive control backgrounds
    static let controlTrack = dynamicColor(
        light: NSColor(red: 0, green: 0, blue: 0, alpha: 0.08),
        dark:  NSColor(red: 1, green: 1, blue: 1, alpha: 0.10)
    )

    // MARK: - Brand (non-adaptive)
    static let teal = Color(red: 0.11, green: 0.61, blue: 0.62)
    static let tealDark = Color(red: 0.08, green: 0.42, blue: 0.43)
    static let tealSoft = Color(red: 0.78, green: 0.91, blue: 0.90)
    static let warning = Color(red: 0.79, green: 0.47, blue: 0.14)

    // MARK: - Physical materials (non-adaptive)
    static let woodLight = Color(red: 0.86, green: 0.67, blue: 0.42)
    static let woodDark = Color(red: 0.52, green: 0.34, blue: 0.18)

    static let panelRadius: CGFloat = 8
    static let controlRadius: CGFloat = 7
    static let toolbarHeight: CGFloat = 54

    enum Spacing {
        static let xs: CGFloat = 6
        static let sm: CGFloat = 10
        static let md: CGFloat = 14
        static let lg: CGFloat = 18
        static let xl: CGFloat = 24
    }

    enum Metrics {
        static let panelPadding: CGFloat = 14
        static let controlHeight: CGFloat = 30
        static let chartHeight: CGFloat = 24
        static let boardPadding: CGFloat = 12
    }
}

enum PanelVariant {
    case elevated
    case bordered
    case glass
    case tonal(Color)
}

struct PanelStyle: ViewModifier {
    var variant: PanelVariant = .elevated

    @ViewBuilder
    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: AppTheme.panelRadius, style: .continuous)

        switch variant {
        case .elevated:
            content
                .background(AppTheme.elevatedSurface, in: shape)
                .overlay(shape.stroke(AppTheme.hairline, lineWidth: 1))
                .shadow(color: .black.opacity(0.04), radius: 10, x: 0, y: 6)
        case .bordered:
            content
                .background(AppTheme.surface, in: shape)
                .overlay(shape.stroke(AppTheme.hairline, lineWidth: 1))
        case .glass:
            content
                .background(AppTheme.glassSurface, in: shape)
                .background(.regularMaterial, in: shape)
                .overlay(shape.stroke(AppTheme.hairline, lineWidth: 1))
                .shadow(color: .black.opacity(0.08), radius: 14, x: 0, y: 8)
        case .tonal(let tint):
            content
                .background(tint.opacity(0.10), in: shape)
                .overlay(shape.stroke(tint.opacity(0.24), lineWidth: 1))
        }
    }
}

extension View {
    func panelStyle(_ variant: PanelVariant = .elevated) -> some View {
        modifier(PanelStyle(variant: variant))
    }
}

struct MetricRow: View {
    let title: String
    let value: String
    var accent: Color = AppTheme.text
    
    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(title)
                .font(.footnote)
                .foregroundStyle(AppTheme.secondaryText)
            Spacer()
            Text(value)
                .font(.subheadline.weight(.semibold))
                .monospacedDigit()
                .foregroundStyle(accent)
        }
    }
}

struct InlineStatusPill: View {
    enum Variant {
        case tonal
        case bordered
        case filled
    }

    let title: String
    let systemImage: String?
    var tint: Color = AppTheme.teal
    var variant: Variant = .tonal

    var body: some View {
        HStack(spacing: 6) {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: 12, weight: .semibold))
            }
            Text(title)
                .font(.footnote.weight(.semibold))
                .lineLimit(1)
        }
        .foregroundStyle(foreground)
        .padding(.horizontal, 9)
        .padding(.vertical, 5)
        .background(background, in: RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous))
        .overlay(shape.stroke(border, lineWidth: 1))
    }

    private var shape: RoundedRectangle {
        RoundedRectangle(cornerRadius: AppTheme.controlRadius, style: .continuous)
    }

    private var foreground: Color {
        switch variant {
        case .filled: .white
        default: tint
        }
    }

    private var background: Color {
        switch variant {
        case .tonal: tint.opacity(0.12)
        case .bordered: Color.clear
        case .filled: tint
        }
    }

    private var border: Color {
        switch variant {
        case .tonal: tint.opacity(0.28)
        case .bordered: tint.opacity(0.44)
        case .filled: tint.opacity(0.0)
        }
    }
}

struct PanelHeader: View {
    let title: String
    var systemImage: String? = nil
    var trailingSystemImage: String? = "chevron.up"

    var body: some View {
        HStack(spacing: 8) {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(AppTheme.tealDark)
            }
            Text(title)
                .font(.callout.weight(.semibold))
                .foregroundStyle(AppTheme.text)
            Spacer()
            if let trailingSystemImage {
                Image(systemName: trailingSystemImage)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(AppTheme.secondaryText)
            }
        }
    }
}

struct StoneToken: View {
    let isBlack: Bool
    var size: CGFloat = 14

    var body: some View {
        Circle()
            .fill(
                RadialGradient(
                    colors: isBlack
                        ? [Color.white.opacity(0.24), Color.black.opacity(0.92), Color.black]
                        : [Color.white, Color(red: 0.90, green: 0.90, blue: 0.88)],
                    center: .topLeading,
                    startRadius: 0,
                    endRadius: size
                )
            )
            .overlay(Circle().stroke(Color.black.opacity(isBlack ? 0.14 : 0.18), lineWidth: 1))
            .shadow(color: .black.opacity(isBlack ? 0.18 : 0.08), radius: 2, x: 0, y: 1)
            .frame(width: size, height: size)
    }
}
