import AppKit

struct BoardRenderer {
    let boardSize: Int
    
    func drawBoard(context: CGContext, size: CGSize,
                   stones: [(col: Int, row: Int, isBlack: Bool)],
                   lastMove: (col: Int, row: Int)?,
                   moveLabels: [(col: Int, row: Int, moveNumber: Int)] = [],
                   showCoordinates: Bool = true,
                   miniMap: Bool = false,
                   suggestions: [(col: Int, row: Int, winrate: Double, lead: Double, visits: Int, order: Int)] = [],
                   territory: [[Bool?]] = []) {
        let side = min(size.width, size.height)
        let origin = CGPoint(x: (size.width - side) / 2, y: (size.height - side) / 2)
        let boardRect = CGRect(origin: origin, size: CGSize(width: side, height: side))
        let padding: CGFloat = miniMap ? max(15, side * 0.09) : side * 0.058
        let gridSize = (side - 2 * padding) / CGFloat(boardSize - 1)
        let stoneRadius = gridSize * 0.44
        
        context.saveGState()
        let radius: CGFloat = miniMap ? 7 : 8
        let path = CGPath(roundedRect: boardRect, cornerWidth: radius, cornerHeight: radius, transform: nil)
        context.addPath(path)
        context.clip()
        
        let woodGradient = CGGradient(
            colorsSpace: CGColorSpaceCreateDeviceRGB(),
            colors: (miniMap
                ? [
                    CGColor(red: 0.90, green: 0.73, blue: 0.49, alpha: 0.92),
                    CGColor(red: 0.76, green: 0.54, blue: 0.31, alpha: 0.92)
                ]
                : [
                    CGColor(red: 0.88, green: 0.70, blue: 0.45, alpha: 1.0),
                    CGColor(red: 0.80, green: 0.58, blue: 0.34, alpha: 1.0)
                ]) as CFArray,
            locations: [0, 1]
        )!
        context.drawLinearGradient(woodGradient, start: CGPoint(x: boardRect.minX, y: boardRect.minY), end: CGPoint(x: boardRect.maxX, y: boardRect.maxY), options: [])

        context.setStrokeColor(CGColor(red: 0.48, green: 0.29, blue: 0.12, alpha: miniMap ? 0.055 : 0.09))
        context.setLineWidth(miniMap ? 0.45 : 0.8)
        for i in 0..<(miniMap ? 18 : 32) {
            if miniMap {
                let y = boardRect.minY + CGFloat(i) * side / 17
                context.move(to: CGPoint(x: boardRect.minX, y: y))
                context.addLine(to: CGPoint(x: boardRect.maxX, y: y + sin(CGFloat(i) * 0.8) * 1.2))
            } else {
                let y = boardRect.minY + CGFloat(i) * side / 31
                context.move(to: CGPoint(x: boardRect.minX, y: y))
                context.addCurve(
                    to: CGPoint(x: boardRect.maxX, y: y + sin(CGFloat(i) * 0.9) * 2.5),
                    control1: CGPoint(x: boardRect.midX - 150, y: y + cos(CGFloat(i)) * 4),
                    control2: CGPoint(x: boardRect.midX + 90, y: y - sin(CGFloat(i)) * 3)
                )
            }
        }
        context.strokePath()
        
        context.setStrokeColor(CGColor(red: 0.18, green: 0.11, blue: 0.05, alpha: miniMap ? 0.34 : 0.66))
        context.setLineWidth(miniMap ? 0.45 : max(0.75, side * 0.0013))
        for i in 0..<boardSize {
            let x = boardRect.minX + padding + CGFloat(i) * gridSize
            context.move(to: CGPoint(x: x, y: boardRect.minY + padding))
            context.addLine(to: CGPoint(x: x, y: boardRect.minY + padding + CGFloat(boardSize - 1) * gridSize))
            let y = boardRect.minY + padding + CGFloat(i) * gridSize
            context.move(to: CGPoint(x: boardRect.minX + padding, y: y))
            context.addLine(to: CGPoint(x: boardRect.minX + padding + CGFloat(boardSize - 1) * gridSize, y: y))
        }
        context.strokePath()

        if showCoordinates {
            drawCoordinates(context: context, boardRect: boardRect, padding: padding, gridSize: gridSize)
        }

        if !territory.isEmpty, !miniMap {
            for r in 0..<boardSize {
                for c in 0..<boardSize {
                    guard let isBlack = territory[r][c] else { continue }
                    let cx = boardRect.minX + padding + CGFloat(c) * gridSize
                    let cy = boardRect.minY + padding + CGFloat(r) * gridSize
                    let markerRadius = gridSize * 0.40
                    let color = isBlack
                        ? CGColor(red: 0.05, green: 0.05, blue: 0.05, alpha: 0.22)
                        : CGColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 0.26)
                    context.setFillColor(color)
                    context.fillEllipse(in: CGRect(x: cx - markerRadius, y: cy - markerRadius,
                                                   width: markerRadius * 2, height: markerRadius * 2))
                }
            }
        }

        let starPoints = [(3,3), (3,9), (3,15), (9,3), (9,9), (9,15), (15,3), (15,9), (15,15)]
        let starRadius = miniMap ? 1.5 : gridSize * 0.08
        context.setFillColor(CGColor(red: 0.12, green: 0.08, blue: 0.04, alpha: miniMap ? 0.44 : 0.82))
        for (col, row) in starPoints {
            let cx = boardRect.minX + padding + CGFloat(col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(row) * gridSize
            context.fillEllipse(in: CGRect(x: cx - starRadius, y: cy - starRadius, width: starRadius * 2, height: starRadius * 2))
        }
        for (col, row, isBlack) in stones {
            let cx = boardRect.minX + padding + CGFloat(col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(row) * gridSize
            context.saveGState()
            context.setShadow(offset: CGSize(width: 0, height: miniMap ? 0.4 : 2.2), blur: miniMap ? 0.8 : 4, color: CGColor(red: 0, green: 0, blue: 0, alpha: miniMap ? 0.10 : 0.24))
            if miniMap {
                context.setFillColor(isBlack ? CGColor(red: 0.13, green: 0.12, blue: 0.11, alpha: 0.74) : CGColor(red: 0.98, green: 0.97, blue: 0.93, alpha: 0.86))
                context.fillEllipse(in: CGRect(x: cx - stoneRadius, y: cy - stoneRadius, width: stoneRadius * 2, height: stoneRadius * 2))
            } else {
                let colors: [CGColor] = isBlack
                    ? [CGColor(red: 0.4, green: 0.4, blue: 0.4, alpha: 1), CGColor(red: 0.05, green: 0.05, blue: 0.05, alpha: 1)]
                    : [CGColor(red: 1, green: 1, blue: 1, alpha: 1), CGColor(red: 0.85, green: 0.85, blue: 0.85, alpha: 1)]
                let gradient = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors as CFArray, locations: [0, 1])!
                context.saveGState()
                context.addEllipse(in: CGRect(x: cx - stoneRadius, y: cy - stoneRadius, width: stoneRadius * 2, height: stoneRadius * 2))
                context.clip()
                context.drawRadialGradient(gradient, startCenter: CGPoint(x: cx - stoneRadius * 0.25, y: cy - stoneRadius * 0.3), startRadius: 0, endCenter: CGPoint(x: cx, y: cy), endRadius: stoneRadius, options: [])
                context.restoreGState()
            }
            context.restoreGState()
        }
        if let (col, row) = lastMove, !miniMap, moveLabels.isEmpty {
            let cx = boardRect.minX + padding + CGFloat(col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(row) * gridSize
            let lastStoneIsBlack = stones.first { $0.col == col && $0.row == row }?.isBlack == true
            context.setStrokeColor(lastStoneIsBlack
                ? CGColor(red: 1, green: 1, blue: 1, alpha: 0.72)
                : CGColor(red: 0.08, green: 0.07, blue: 0.06, alpha: 0.54)
            )
            context.setLineWidth(max(1.2, gridSize * 0.055))
            context.strokeEllipse(in: CGRect(x: cx - stoneRadius * 0.34, y: cy - stoneRadius * 0.34, width: stoneRadius * 0.68, height: stoneRadius * 0.68))
        }
        if !moveLabels.isEmpty {
            drawMoveLabels(context: context, boardRect: boardRect, padding: padding, gridSize: gridSize, stones: stones, moveLabels: moveLabels, miniMap: miniMap)
        }

        if !suggestions.isEmpty, !miniMap {
            let maxVisits = max(1, suggestions.map(\.visits).max() ?? 1)
            let pulse = (sin(CGFloat(Date.timeIntervalSinceReferenceDate) * 4.8) + 1.0) * 0.5
            for sug in suggestions {
                let cx = boardRect.minX + padding + CGFloat(sug.col) * gridSize
                let cy = boardRect.minY + padding + CGFloat(sug.row) * gridSize
                let visitWeight = CGFloat(sug.visits) / CGFloat(maxVisits)
                let baseRadius = stoneRadius * (0.58 + 0.46 * sqrt(max(0.08, visitWeight)))
                let ringRadius = baseRadius * (1.0 + 0.10 * pulse)
                let winrate = CGFloat(min(max(sug.winrate, 0.0), 1.0))
                let warm = winrate < 0.5 ? (0.5 - winrate) * 2.0 : 0
                let cool = winrate >= 0.5 ? (winrate - 0.5) * 2.0 : 0
                let color = CGColor(
                    red: 0.18 + 0.62 * warm,
                    green: 0.42 + 0.18 * min(warm, cool),
                    blue: 0.52 + 0.36 * cool,
                    alpha: sug.order == 0 ? 0.82 : 0.60
                )

                context.setShadow(offset: CGSize(width: 0, height: 1.5), blur: 4.0, color: color.copy(alpha: 0.28))
                context.setStrokeColor(color)
                context.setLineWidth(max(2.0, gridSize * (0.050 + 0.020 * visitWeight)))
                context.strokeEllipse(in: CGRect(x: cx - ringRadius, y: cy - ringRadius,
                                                 width: ringRadius * 2, height: ringRadius * 2))

                context.setShadow(offset: .zero, blur: 0, color: nil)
                context.setFillColor(color.copy(alpha: 0.16) ?? color)
                let fillRadius = baseRadius * 0.62
                context.fillEllipse(in: CGRect(x: cx - fillRadius, y: cy - fillRadius,
                                               width: fillRadius * 2, height: fillRadius * 2))

                if sug.order == 0 {
                    context.setStrokeColor(CGColor(red: 1.0, green: 1.0, blue: 0.92, alpha: 0.78))
                    context.setLineWidth(max(1.2, gridSize * 0.035))
                    context.strokeEllipse(in: CGRect(x: cx - ringRadius * 0.78, y: cy - ringRadius * 0.78,
                                                     width: ringRadius * 1.56, height: ringRadius * 1.56))
                }
            }
        }

        context.restoreGState()
    }

    private func drawCoordinates(context: CGContext, boardRect: CGRect, padding: CGFloat, gridSize: CGFloat) {
        let fontSize = max(7, min(12, gridSize * 0.34))
        let font = CTFontCreateUIFontForLanguage(.system, fontSize, nil) ?? CTFontCreateWithName(".AppleSystemUIFont" as CFString, fontSize, nil)
        let attrs: [CFString: Any] = [
            kCTFontAttributeName: font,
            kCTForegroundColorAttributeName: CGColor(red: 0.12, green: 0.09, blue: 0.06, alpha: 0.50)
        ]
        let columns = Array("ABCDEFGHJKLMNOPQRST").map(String.init)
        for i in 0..<boardSize {
            let x = boardRect.minX + padding + CGFloat(i) * gridSize
            drawCenteredText(columns[i], at: CGPoint(x: x, y: boardRect.maxY - padding * 0.50), attrs: attrs, context: context)
            drawCenteredText(columns[i], at: CGPoint(x: x, y: boardRect.minY + padding * 0.28), attrs: attrs, context: context)

            let label = "\(boardSize - i)"
            let y = boardRect.minY + padding + CGFloat(i) * gridSize
            drawCenteredText(label, at: CGPoint(x: boardRect.minX + padding * 0.38, y: y), attrs: attrs, context: context)
            drawCenteredText(label, at: CGPoint(x: boardRect.maxX - padding * 0.38, y: y), attrs: attrs, context: context)
        }
    }

    private func drawMoveLabels(context: CGContext, boardRect: CGRect, padding: CGFloat, gridSize: CGFloat, stones: [(col: Int, row: Int, isBlack: Bool)], moveLabels: [(col: Int, row: Int, moveNumber: Int)], miniMap: Bool) {
        let fontSize = miniMap ? max(5.5, gridSize * 0.34) : max(9, min(15, gridSize * 0.36))
        let font = CTFontCreateUIFontForLanguage(.emphasizedSystem, fontSize, nil) ?? CTFontCreateWithName(".AppleSystemUIFontEmphasized" as CFString, fontSize, nil)

        for label in moveLabels {
            let cx = boardRect.minX + padding + CGFloat(label.col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(label.row) * gridSize
            let stone = stones.first { $0.col == label.col && $0.row == label.row }
            let textColor: CGColor
            if stone?.isBlack == true {
                textColor = CGColor(red: 0.98, green: 0.98, blue: 0.96, alpha: miniMap ? 0.86 : 0.94)
            } else {
                textColor = CGColor(red: 0.10, green: 0.09, blue: 0.08, alpha: miniMap ? 0.68 : 0.84)
            }
            let attrs: [CFString: Any] = [
                kCTFontAttributeName: font,
                kCTForegroundColorAttributeName: textColor
            ]
            drawCenteredText("\(label.moveNumber)", at: CGPoint(x: cx, y: cy), attrs: attrs, context: context)
        }
    }

    private func drawCenteredText(_ text: String, at point: CGPoint, attrs: [CFString: Any], context: CGContext) {
        guard let attributed = CFAttributedStringCreate(nil, text as CFString, attrs as CFDictionary) else { return }
        let line = CTLineCreateWithAttributedString(attributed)
        var ascent: CGFloat = 0
        var descent: CGFloat = 0
        var leading: CGFloat = 0
        let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
        context.textPosition = CGPoint(x: point.x - width / 2, y: point.y - (ascent - descent) / 2)
        CTLineDraw(line, context)
    }
}
