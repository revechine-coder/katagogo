import Foundation

final class GameStore {
    static let shared = GameStore()

    private let fileManager = FileManager.default
    private let gamesDir: URL
    private let indexPath: URL

    init() {
        let docs = fileManager.urls(for: .documentDirectory, in: .userDomainMask).first!
        gamesDir = docs.appendingPathComponent("katagogo/games", isDirectory: true)
        indexPath = gamesDir.appendingPathComponent("_index.json")
        try? fileManager.createDirectory(at: gamesDir, withIntermediateDirectories: true)
    }

    func save(_ game: SavedGameFile) throws {
        let id = gameFileName(from: game.gameInfo.savedAt)
        let fileURL = gamesDir.appendingPathComponent("\(id).json")
        let data = try JSONEncoder().encode(game)
        try data.write(to: fileURL)
        updateIndexAfterSaving(id: id, game: game)
    }

    func load(id: String) throws -> SavedGameFile {
        let fileURL = gamesDir.appendingPathComponent("\(id).json")
        let data = try Data(contentsOf: fileURL)
        return try JSONDecoder().decode(SavedGameFile.self, from: data)
    }

    func delete(id: String) throws {
        let fileURL = gamesDir.appendingPathComponent("\(id).json")
        try fileManager.removeItem(at: fileURL)
        removeFromIndex(id: id)
    }

    func listGames() -> [GameIndexEntry] {
        guard let data = try? Data(contentsOf: indexPath),
              let entries = try? JSONDecoder().decode([GameIndexEntry].self, from: data)
        else { return [] }
        return entries.sorted { $0.savedAt > $1.savedAt }
    }

    private func gameFileName(from date: Date) -> String {
        let formatter = ISO8601DateFormatter()
        return formatter.string(from: date).replacingOccurrences(of: ":", with: "-")
    }

    private func readIndex() -> [GameIndexEntry] {
        guard let data = try? Data(contentsOf: indexPath),
              let entries = try? JSONDecoder().decode([GameIndexEntry].self, from: data)
        else { return [] }
        return entries
    }

    private func writeIndex(_ entries: [GameIndexEntry]) {
        guard let data = try? JSONEncoder().encode(entries) else { return }
        try? data.write(to: indexPath)
    }

    private func updateIndexAfterSaving(id: String, game: SavedGameFile) {
        var entries = readIndex()
        entries.removeAll { $0.id == id }
        entries.append(GameIndexEntry(
            id: id,
            savedAt: game.gameInfo.savedAt,
            humanColor: game.gameInfo.humanColor,
            moveCount: game.moves.count,
            finalScore: game.finalScore,
            engineModel: game.gameInfo.engineModel
        ))
        writeIndex(entries)
    }

    private func removeFromIndex(id: String) {
        var entries = readIndex()
        entries.removeAll { $0.id == id }
        writeIndex(entries)
    }
}

struct GameIndexEntry: Codable {
    let id: String
    let savedAt: Date
    let humanColor: String
    let moveCount: Int
    var finalScore: String?
    let engineModel: String
}
