import SwiftUI
import Observation

enum GamePhase {
    case idle
    case connecting
    case playing
    case waiting
    case finished
}

@Observable
final class GameViewModel {
    
    var phase: GamePhase = .idle
    var currentPlayer: String = "b"
    var moveCount: Int = 0
    var winrateBlack: Double = 0.5
    var lead: Double = 0.0
    var capturesBlack: Int = 0
    var capturesWhite: Int = 0
    var lastMove: (col: Int, row: Int)? = nil
    var errorMessage: String? = nil
    
    var board: [[Bool?]] = Array(repeating: Array(repeating: nil, count: 19), count: 19)
    var moveLabels: [(col: Int, row: Int, moveNumber: Int)] = []
    
    private let engine = GoCoreBridge()
    
    private let columns = "ABCDEFGHJKLMNOPQRST"
    
    func startGame() {
        guard phase == .idle else { return }

        let appState = AppState.shared
        guard appState.isConfigured else {
            errorMessage = "请先在菜单中配置 KataGo 路径"
            return
        }

        let binaryPath = appState.kataGoBinaryPath
        let configPath = appState.kataGoConfigPath
        let modelPath = appState.kataGoModelPath
        let boardSize = appState.boardSize
        let aiLevel = appState.aiLevel

        phase = .connecting

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }

            guard self.engine.create() else {
                DispatchQueue.main.async {
                    self.errorMessage = "Failed to create engine"
                    self.phase = .idle
                }
                return
            }

            do {
                try self.engine.start(
                    binaryPath: binaryPath,
                    configPath: configPath,
                    modelPath: modelPath,
                    boardSize: boardSize,
                    timeout: 120.0
                )
                self.engine.setLevel(aiLevel)
                DispatchQueue.main.async {
                    self.phase = .playing
                }
            } catch {
                DispatchQueue.main.async {
                    self.errorMessage = error.localizedDescription
                    self.phase = .idle
                }
            }
        }
    }
    
    
    func endGame() {
        engine.close()
        engine.destroy()
        phase = .idle
    }
    
    func play(at col: Int, row: Int) {
        guard phase == .playing else { return }
        guard col >= 0, col < 19, row >= 0, row < 19 else { return }
        guard board[row][col] == nil else { return }
        
        let vertex = vertexFromPoint(col: col, row: row)
        phase = .waiting
        
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }
            
            do {
                try self.engine.play(color: self.currentPlayer, vertex: vertex)
                let aiColor = self.currentPlayer == "b" ? "w" : "b"
                _ = try self.engine.genmove(color: aiColor)
                
                DispatchQueue.main.async {
                    self.refreshUI()
                    self.phase = .playing
                }
            } catch {
                DispatchQueue.main.async {
                    self.phase = .playing
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }
    
    func undo() {
        guard phase == .playing else { return }
        _ = engine.undo()
        refreshUI()
    }
    
    func setLevel(_ level: Int) {
        engine.setLevel(level)
        AppState.shared.aiLevel = level
    }
    
    func resetGame() {
        do {
            try engine.reset()
            board = Array(repeating: Array(repeating: nil, count: 19), count: 19)
            moveLabels = []
            moveCount = 0
            currentPlayer = "b"
            winrateBlack = 0.5
            lead = 0.0
            lastMove = nil
            phase = .playing
        } catch {
            errorMessage = error.localizedDescription
        }
    }
    
    private func refreshUI() {
        guard let frame = engine.getRenderFrame() else { return }
        
        var updatedBoard = Array(repeating: Array(repeating: Optional<Bool>.none, count: frame.boardSize), count: frame.boardSize)
        for stone in frame.stones {
            updatedBoard[stone.row][stone.col] = stone.isBlack
        }
        
        board = updatedBoard
        moveLabels = frame.moveLabels
        moveCount = frame.moveCount
        currentPlayer = frame.currentPlayer
        capturesBlack = frame.capturesBlack
        capturesWhite = frame.capturesWhite
        lastMove = frame.lastMove
        
        if let analysis = engine.getAnalysis() {
            winrateBlack = analysis.winrateBlack
            lead = analysis.lead
            moveCount = analysis.moveCount
            currentPlayer = analysis.currentPlayer
            capturesBlack = analysis.capturesBlack
            capturesWhite = analysis.capturesWhite
        }
    }
    
    private func vertexFromPoint(col: Int, row: Int) -> String {
        let column = columns[columns.index(columns.startIndex, offsetBy: col)]
        return "\(column)\(19 - row)"
    }
}
