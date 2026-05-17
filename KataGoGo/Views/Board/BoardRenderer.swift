import AppKit

struct BoardRenderer {
    let boardSize: Int
    
    func drawBoard(context: CGContext, size: CGSize,
                   stones: [(col: Int, row: Int, isBlack: Bool)],
                   lastMove: (col: Int, row: Int)?,
                   moveLabels: [(col: Int, row: Int, moveNumber: Int)] = [],
                   showCoordinates: Bool = true,
                   miniMap: Bool = false) {
        let side = min(size.width, size.height)
        let origin = CGPoint(x: (size.width - side) / 2, y: (size.height - side) / 2)
        let boardRect = CGRect(origin: origin, size: CGSize(width: side, height: side))
        let padding: CGFloat = miniMap ? 5 : side * 0.065
        let gridSize = (side - 2 * padding) / CGFloat(boardSize - 1)
        let stoneRadius = gridSize * 0.44
        
        context.saveGState()
        let radius: CGFloat = miniMap ? 7 : 8
        let path = CGPath(roundedRect: boardRect, cornerWidth: radius, cornerHeight: radius, transform: nil)
        context.addPath(path)
        context.clip()
        
        let woodGradient = CGGradient(
            colorsSpace: CGColorSpaceCreateDeviceRGB(),
            colors: [
                CGColor(red: 0.91, green: 0.72, blue: 0.46, alpha: 1.0),
                CGColor(red: 0.75, green: 0.50, blue: 0.26, alpha: 1.0)
            ] as CFArray,
            locations: [0, 1]
        )!
        context.drawLinearGradient(woodGradient, start: CGPoint(x: boardRect.minX, y: boardRect.minY), end: CGPoint(x: boardRect.maxX, y: boardRect.maxY), options: [])
        
        if !miniMap {
            context.setStrokeColor(CGColor(red: 0.38, green: 0.22, blue: 0.10, alpha: 0.12))
            context.setLineWidth(1)
            for i in 0..<20 {
                let y = boardRect.minY + CGFloat(i) * side / 19
                context.move(to: CGPoint(x: boardRect.minX, y: y))
                context.addCurve(
                    to: CGPoint(x: boardRect.maxX, y: y + sin(CGFloat(i)) * 2),
                    control1: CGPoint(x: boardRect.midX - 120, y: y + 5),
                    control2: CGPoint(x: boardRect.midX + 80, y: y - 4)
                )
            }
            context.strokePath()
        }
        
        context.setStrokeColor(CGColor(red: 0.16, green: 0.10, blue: 0.05, alpha: miniMap ? 0.72 : 0.82))
        context.setLineWidth(miniMap ? 0.55 : 1.05)
        for i in 0..<boardSize {
            let x = boardRect.minX + padding + CGFloat(i) * gridSize
            context.move(to: CGPoint(x: x, y: boardRect.minY + padding))
            context.addLine(to: CGPoint(x: x, y: boardRect.minY + padding + CGFloat(boardSize - 1) * gridSize))
            let y = boardRect.minY + padding + CGFloat(i) * gridSize
            context.move(to: CGPoint(x: boardRect.minX + padding, y: y))
            context.addLine(to: CGPoint(x: boardRect.minX + padding + CGFloat(boardSize - 1) * gridSize, y: y))
        }
        context.strokePath()
        let starPoints = [(3,3), (3,9), (3,15), (9,3), (9,9), (9,15), (15,3), (15,9), (15,15)]
        let starRadius = miniMap ? 1.5 : gridSize * 0.08
        context.setFillColor(CGColor(red: 0.12, green: 0.08, blue: 0.04, alpha: 0.9))
        for (col, row) in starPoints {
            let cx = boardRect.minX + padding + CGFloat(col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(row) * gridSize
            context.fillEllipse(in: CGRect(x: cx - starRadius, y: cy - starRadius, width: starRadius * 2, height: starRadius * 2))
        }
        for (col, row, isBlack) in stones {
            let cx = boardRect.minX + padding + CGFloat(col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(row) * gridSize
            if miniMap {
                context.setFillColor(isBlack ? CGColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 1) : CGColor(red: 1, green: 1, blue: 1, alpha: 1))
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
        }
        if let (col, row) = lastMove, !miniMap {
            let cx = boardRect.minX + padding + CGFloat(col) * gridSize
            let cy = boardRect.minY + padding + CGFloat(row) * gridSize
            context.setStrokeColor(CGColor(red: 0.06, green: 0.44, blue: 0.42, alpha: 0.62))
            context.setLineWidth(1.4)
            context.strokeEllipse(in: CGRect(x: cx - stoneRadius * 0.42, y: cy - stoneRadius * 0.42, width: stoneRadius * 0.84, height: stoneRadius * 0.84))
        }
        if miniMap {
            let fontSize = max(6, gridSize * 0.35)
            let font = CTFontCreateWithName("Helvetica" as CFString, fontSize, nil)
            let attrs: [CFString: Any] = [kCTFontAttributeName: font, kCTForegroundColorAttributeName: CGColor(red: 0.1, green: 0.1, blue: 0.1, alpha: 0.7)]
            for label in moveLabels {
                let cx = boardRect.minX + padding + CGFloat(label.col) * gridSize, cy = boardRect.minY + padding + CGFloat(label.row) * gridSize
                let asCF = "\(label.moveNumber)" as CFString
                let asCFAttr = CFAttributedStringCreate(nil, asCF, attrs as CFDictionary)
                let line = CTLineCreateWithAttributedString(asCFAttr!)
                let b = CTLineGetBoundsWithOptions(line, [])
                context.textPosition = CGPoint(x: cx - b.width / 2, y: cy - b.height / 2)
                CTLineDraw(line, context)
            }
        }
        context.restoreGState()
    }
}
