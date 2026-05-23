import AppKit
import Foundation

let size = 1024.0
let scale = size / 30.0  // AppMark original is 30x30

// ── Colors (from DesignSystem) ──
let woodLight = CGColor(red: 0.86, green: 0.67, blue: 0.42, alpha: 1)
let woodDark  = CGColor(red: 0.52, green: 0.34, blue: 0.18, alpha: 1)
let tealSoft  = CGColor(red: 0.78, green: 0.91, blue: 0.90, alpha: 1)
let teal      = CGColor(red: 0.11, green: 0.61, blue: 0.62, alpha: 1)
let tealDark  = CGColor(red: 0.08, green: 0.42, blue: 0.43, alpha: 1)

let whiteStoneTop    = CGColor(red: 1.00, green: 1.00, blue: 1.00, alpha: 0.94)
let whiteStoneBottom = CGColor(red: 0.90, green: 0.90, blue: 0.88, alpha: 1)
let blackStoneTop    = CGColor(red: 0.40, green: 0.40, blue: 0.40, alpha: 1)
let blackStoneBottom = CGColor(red: 0.05, green: 0.05, blue: 0.05, alpha: 1)

let gridLineColor = CGColor(red: 0, green: 0, blue: 0, alpha: 0.12)
let borderColor   = CGColor(red: 0, green: 0, blue: 0, alpha: 0.20)

// ── Create bitmap context ──
guard let ctx = CGContext(
    data: nil,
    width: Int(size), height: Int(size),
    bitsPerComponent: 8,
    bytesPerRow: 0,
    space: CGColorSpaceCreateDeviceRGB(),
    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
) else {
    print("ERROR: Could not create bitmap context")
    exit(1)
}

let rect = CGRect(x: 0, y: 0, width: size, height: size)

// ── 1. Wood background with rounded corners ──
let cornerRadius = size * 0.225  // macOS icon corner radius
let bgPath = CGPath(roundedRect: rect, cornerWidth: cornerRadius, cornerHeight: cornerRadius, transform: nil)
ctx.addPath(bgPath)
ctx.clip()

let woodGradient = CGGradient(
    colorsSpace: CGColorSpaceCreateDeviceRGB(),
    colors: [woodLight, woodDark] as CFArray,
    locations: [0, 1]
)!
ctx.drawLinearGradient(woodGradient,
    start: CGPoint(x: 0, y: size),
    end: CGPoint(x: size, y: 0),
    options: [])

// ── 2. Small grid pattern (mini go board) ──
let gridCount = 19
let gridPadding = size * 0.10
let gridStep = (size - 2 * gridPadding) / CGFloat(gridCount - 1)
ctx.setStrokeColor(gridLineColor)
ctx.setLineWidth(max(0.8, scale * 0.15))
for i in 0..<gridCount {
    let pos = gridPadding + CGFloat(i) * gridStep
    ctx.move(to: CGPoint(x: pos, y: gridPadding))
    ctx.addLine(to: CGPoint(x: pos, y: size - gridPadding))
    ctx.move(to: CGPoint(x: gridPadding, y: pos))
    ctx.addLine(to: CGPoint(x: size - gridPadding, y: pos))
}
ctx.strokePath()

// ── 3. Star points ──
let starPoints = [(3,3),(3,9),(3,15),(9,3),(9,9),(9,15),(15,3),(15,9),(15,15)]
let starRadius = max(2.5, scale * 0.15)
ctx.setFillColor(CGColor(red: 0.12, green: 0.08, blue: 0.04, alpha: 0.45))
for (col, row) in starPoints {
    let cx = gridPadding + CGFloat(col) * gridStep
    let cy = gridPadding + CGFloat(row) * gridStep
    ctx.fillEllipse(in: CGRect(x: cx - starRadius, y: cy - starRadius,
                                width: starRadius * 2, height: starRadius * 2))
}

// ── 4. Black stone (top-left area) ──
let stoneR = size * 0.21
let blackCenter = CGPoint(x: size * 0.32, y: size * 0.30)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: scale * 2.2), blur: scale * 4,
              color: CGColor(red: 0, green: 0, blue: 0, alpha: 0.3))
let blackStoneGradient = CGGradient(
    colorsSpace: CGColorSpaceCreateDeviceRGB(),
    colors: [blackStoneTop, blackStoneBottom] as CFArray,
    locations: [0, 1]
)!
ctx.addEllipse(in: CGRect(x: blackCenter.x - stoneR, y: blackCenter.y - stoneR,
                           width: stoneR * 2, height: stoneR * 2))
ctx.clip()
ctx.drawRadialGradient(blackStoneGradient,
    startCenter: CGPoint(x: blackCenter.x - stoneR * 0.25, y: blackCenter.y - stoneR * 0.3),
    startRadius: 0,
    endCenter: blackCenter,
    endRadius: stoneR,
    options: [])
ctx.restoreGState()

// ── 5. White stone (bottom-right area, partially overlapping) ──
let whiteCenter = CGPoint(x: size * 0.70, y: size * 0.72)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: scale * 2.2), blur: scale * 4,
              color: CGColor(red: 0, green: 0, blue: 0, alpha: 0.18))
let whiteStoneGradient = CGGradient(
    colorsSpace: CGColorSpaceCreateDeviceRGB(),
    colors: [whiteStoneTop, whiteStoneBottom] as CFArray,
    locations: [0, 1]
)!
ctx.addEllipse(in: CGRect(x: whiteCenter.x - stoneR, y: whiteCenter.y - stoneR,
                           width: stoneR * 2, height: stoneR * 2))
ctx.clip()
ctx.drawRadialGradient(whiteStoneGradient,
    startCenter: CGPoint(x: whiteCenter.x - stoneR * 0.25, y: whiteCenter.y - stoneR * 0.3),
    startRadius: 0,
    endCenter: whiteCenter,
    endRadius: stoneR,
    options: [])
ctx.restoreGState()

// White stone border
ctx.setStrokeColor(CGColor(red: 0, green: 0, blue: 0, alpha: 0.12))
ctx.setLineWidth(max(1.5, scale * 0.1))
ctx.strokeEllipse(in: CGRect(x: whiteCenter.x - stoneR, y: whiteCenter.y - stoneR,
                               width: stoneR * 2, height: stoneR * 2))

// ── 6. Sparkle accent (tealSoft) ──
let sparkleCenter = CGPoint(x: size * 0.72, y: size * 0.26)
let sparkleR = size * 0.09

// Outer glow
ctx.saveGState()
ctx.setShadow(offset: .zero, blur: scale * 8,
              color: CGColor(red: 0.08, green: 0.42, blue: 0.43, alpha: 0.5))
ctx.setFillColor(tealSoft)
ctx.fillEllipse(in: CGRect(x: sparkleCenter.x - sparkleR, y: sparkleCenter.y - sparkleR,
                            width: sparkleR * 2, height: sparkleR * 2))
ctx.restoreGState()

// Sparkle rays (4-point star)
let rayLength = sparkleR * 1.5
let rayWidth = sparkleR * 0.3
ctx.setFillColor(teal)
for angle in [0, CGFloat.pi / 2] {
    ctx.saveGState()
    ctx.translateBy(x: sparkleCenter.x, y: sparkleCenter.y)
    ctx.rotate(by: angle)
    let ray = CGPath(roundedRect: CGRect(x: -rayWidth / 2, y: -rayLength,
                                          width: rayWidth, height: rayLength * 2),
                      cornerWidth: rayWidth / 2, cornerHeight: rayWidth / 2,
                      transform: nil)
    ctx.addPath(ray)
    ctx.fillPath()
    ctx.restoreGState()
}
for angle in [CGFloat.pi / 4, 3 * CGFloat.pi / 4] {
    ctx.saveGState()
    ctx.translateBy(x: sparkleCenter.x, y: sparkleCenter.y)
    ctx.rotate(by: angle)
    let ray = CGPath(roundedRect: CGRect(x: -rayWidth * 0.6 / 2, y: -rayLength * 0.7,
                                          width: rayWidth * 0.6, height: rayLength * 1.4),
                      cornerWidth: rayWidth * 0.3, cornerHeight: rayWidth * 0.3,
                      transform: nil)
    ctx.addPath(ray)
    ctx.fillPath()
    ctx.restoreGState()
}

// Center dot
ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 0.9))
ctx.fillEllipse(in: CGRect(x: sparkleCenter.x - sparkleR * 0.25, y: sparkleCenter.y - sparkleR * 0.25,
                            width: sparkleR * 0.5, height: sparkleR * 0.5))

// ── 7. Border ──
ctx.resetClip()
ctx.setStrokeColor(borderColor)
ctx.setLineWidth(max(3, scale * 0.3))
ctx.addPath(bgPath)
ctx.strokePath()

// ── Save as PNG ──
guard let image = ctx.makeImage() else {
    print("ERROR: Could not create image from context")
    exit(1)
}

let desktop = FileManager.default.homeDirectoryForCurrentUser
    .appendingPathComponent("Desktop")
    .appendingPathComponent("KataGoGo_Icon_1024.png")

let rep = NSBitmapImageRep(cgImage: image)
rep.size = NSSize(width: size, height: size)
guard let pngData = rep.representation(using: .png, properties: [:]) else {
    print("ERROR: Could not create PNG data")
    exit(1)
}

do {
    try pngData.write(to: desktop)
    print("Icon saved to: \(desktop.path)")
    print("Size: \(size)x\(size)")
} catch {
    print("ERROR: \(error)")
    exit(1)
}
