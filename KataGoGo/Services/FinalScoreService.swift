import Foundation

struct FinalScoreResult {
    let rawValue: String
    let displayText: String
}

enum FinalScoreService {
    static func makeResult(from rawValue: String) -> FinalScoreResult {
        let trimmed = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return FinalScoreResult(rawValue: rawValue, displayText: "算目无结果")
        }

        if trimmed == "0" || trimmed.lowercased() == "draw" {
            return FinalScoreResult(rawValue: trimmed, displayText: "终局算目：双方持平")
        }

        let winnerText: String
        if trimmed.hasPrefix("B+") {
            winnerText = "黑胜"
        } else if trimmed.hasPrefix("W+") {
            winnerText = "白胜"
        } else {
            return FinalScoreResult(rawValue: trimmed, displayText: "终局算目：\(trimmed)")
        }

        let margin = String(trimmed.dropFirst(2))
        if margin.caseInsensitiveCompare("Resign") == .orderedSame {
            return FinalScoreResult(rawValue: trimmed, displayText: "终局算目：\(winnerText)，对方认输")
        }

        return FinalScoreResult(rawValue: trimmed, displayText: "终局算目：\(winnerText) \(margin) 目")
    }
}
