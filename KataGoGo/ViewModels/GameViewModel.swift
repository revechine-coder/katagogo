import SwiftUI
import Combine

enum GamePhase {
    case idle
    case connecting
    case playing
    case paused
    case waiting
    case finished
}

final class GameViewModel: ObservableObject {

    @Published var phase: GamePhase = .idle
    @Published var currentPlayer: String = "b"
    @Published var moveCount: Int = 0
    @Published var winrateBlack: Double = 0.5
    @Published var leadBlack: Double = 0.0
    @Published var evaluationAccuracy: Double = 0.0
    @Published var capturesBlack: Int = 0
    @Published var capturesWhite: Int = 0
    @Published var lastMove: (col: Int, row: Int)? = nil
    @Published var modelName: String = AppState.shared.modelDisplayName
    @Published var handicapCount: Int = AppState.shared.handicapCount
    @Published var humanColor: String = "b"
    @Published var lastThinkDuration: TimeInterval? = nil
    @Published var errorMessage: String? = nil
    @Published var finalScoreText: String? = nil
    @Published var isScoringFinalResult = false
    @Published var showsMoveLabelsOnMainBoard: Bool = false
    @Published var showsCoordinatesOnMainBoard: Bool = false
    @Published var territory: [[Bool?]] = []

    @Published var board: [[Bool?]] = Array(repeating: Array(repeating: nil, count: 19), count: 19)
    @Published var moveLabels: [(col: Int, row: Int, moveNumber: Int)] = []
    @Published var moveSuggestions: [(col: Int, row: Int, winrate: Double, order: Int)] = []
    @Published var isShowingSuggestions = false

    var winrateWhite: Double { 1.0 - winrateBlack }
    var leadWhite: Double { -leadBlack }

    @Published var gameStartTime: Date? = nil
    @Published var accumulatedElapsed: TimeInterval = 0
    @Published var moveElapsedAtMove: [TimeInterval] = []
    @Published var moveAnalysisHistory: [MoveAnalysisSnapshot] = []
    @Published var isReviewMode: Bool = false
    @Published var replayTargetMove: Int = 0
    var totalRecordedMoves: Int { recordedMoves.count }
    private var reviewRecordedMovesSnapshot: [SavedMove] = []
    private var reviewMoveElapsedSnapshot: [TimeInterval] = []
    @Published var displayTick: Int = 0
    @Published var boardVersion: Int = 0

    var totalElapsedText: String {
        _ = displayTick
        let total = totalElapsedSeconds
        let seconds = Int(total)
        return String(format: "%02d:%02d:%02d", seconds / 3600, (seconds / 60) % 60, seconds % 60)
    }

    var totalElapsedSeconds: TimeInterval {
        var total = accumulatedElapsed
        if let start = gameStartTime {
            total += Date().timeIntervalSince(start)
        }
        return total
    }

    func moveElapsedText(for moveNumber: Int) -> String {
        let idx = moveNumber - 1
        guard idx >= 0, idx < moveElapsedAtMove.count else { return "-" }
        let seconds = Int(moveElapsedAtMove[idx])
        if seconds >= 3600 {
            return String(format: "%d:%02d:%02d", seconds / 3600, (seconds / 60) % 60, seconds % 60)
        }
        return String(format: "%02d:%02d", seconds / 60, seconds % 60)
    }

    private let engine = GoCoreBridge()
    private let engineSettingsQueue = DispatchQueue(label: "winner.KataGoGo.engineSettings", qos: .userInitiated)
    private var isEngineRunning = false
    @Published var recordedMoves: [SavedMove] = []
    private var restoredPausedSession: SavedGameSession?
    private var displayTimer: Timer? = nil

    private let columns = "ABCDEFGHJKLMNOPQRST"

    init() {
        restorePausedSessionIfAvailable()
    }

    func startGame() {
        guard phase == .idle else { return }
        startEngineForCurrentGame()
    }

    private func startEngineForCurrentGame(replaying session: SavedGameSession? = nil, onReady: (() -> Void)? = nil) {
        guard phase == .idle || phase == .paused else { return }

        if isEngineRunning {
            do {
                try engine.setHandicap(handicapCount)
                clearSavedSession()
                refreshUI()
                phase = .playing
                startDisplayTimer()
                onReady?()
                if session == nil && shouldRequestOpeningMove && onReady == nil {
                    requestOpeningMove()
                }
            } catch {
                errorMessage = error.localizedDescription
            }
            return
        }

        let appState = AppState.shared
        guard appState.isConfigured else {
            errorMessage = appState.configurationError ?? "请先配置 KataGo 引擎"
            return
        }

        let binaryPath = appState.kataGoBinaryPath
        let configPath = appState.kataGoConfigPath
        let modelPath = appState.kataGoModelPath
        let boardSize = appState.boardSize
        let handicapCount = appState.handicapCount

        modelName = appState.modelDisplayName
        self.handicapCount = handicapCount
        if handicapCount > 0 {
            humanColor = "b"
        }
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
                try self.engine.setHandicap(session?.handicapCount ?? handicapCount)
                if let session {
                    try self.replay(session.moves)
                }
                Task { @MainActor [weak self] in
                    guard let self = self else { return }
                    self.phase = .playing
                    self.isEngineRunning = true
                    if let session {
                        self.recordedMoves = session.moves
                        self.restoredPausedSession = nil
                        self.clearSavedSession()
                    }
                    self.refreshUI()
                    self.startDisplayTimer()
                    onReady?()
                    if session == nil && self.shouldRequestOpeningMove && onReady == nil {
                        self.requestOpeningMove()
                    }
                }
            } catch {
                Task { @MainActor [weak self] in
                    self?.errorMessage = error.localizedDescription
                    self?.phase = .idle
                }
            }
        }
    }

    func startOrResumeGame() {
        switch phase {
        case .idle:
            startGame()
        case .paused:
            if let restoredPausedSession, !isEngineRunning {
                startEngineForCurrentGame(replaying: restoredPausedSession)
            } else {
                clearSavedSession()
                phase = .playing
                startDisplayTimer()
            }
        default:
            break
        }
    }

    func pauseGame() {
        guard phase == .playing else { return }
        accumulateElapsedAndPauseTimer()
        phase = .paused
        savePausedSession()
    }


    func endGame() {
        stopDisplayTimer()
        engine.close()
        engine.destroy()
        isEngineRunning = false
        phase = .idle
    }

    func play(at col: Int, row: Int) {
        if phase == .idle && humanColor == "b" {
            startEngineForCurrentGame {
                self.play(at: col, row: row)
            }
            return
        }

        guard phase == .playing else { return }
        guard currentPlayer == humanColor else { return }
        guard col >= 0, col < 19, row >= 0, row < 19 else { return }
        guard board[row][col] == nil else { return }

        let vertex = vertexFromPoint(col: col, row: row)
        let humanMoveNumber = moveCount + 1
        let humanMove = SavedMove(color: currentPlayer, vertex: vertex)
        let aiColor = currentPlayer == "b" ? "w" : "b"

        board[row][col] = currentPlayer == "b"
        moveLabels.append((col: col, row: row, moveNumber: humanMoveNumber))
        moveCount = humanMoveNumber
        boardVersion &+= 1
        lastMove = (col, row)
        StoneSoundService.shared.playClick()
        currentPlayer = aiColor
        moveSuggestions = []
        moveElapsedAtMove.append(totalElapsedSeconds)
        phase = .waiting
        territory = []

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }

            do {
                try self.engine.play(color: humanMove.color, vertex: vertex)
                let thinkStart = Date()
                let aiMove = try self.engine.genmove(color: aiColor)
                let thinkDuration = Date().timeIntervalSince(thinkStart)

                DispatchQueue.main.async {
                    self.recordedMoves.append(humanMove)
                    if aiMove.vertex.uppercased() != "RESIGN" {
                        self.recordedMoves.append(SavedMove(color: aiColor, vertex: aiMove.vertex))
                    }
                    self.lastThinkDuration = thinkDuration
                    self.moveElapsedAtMove.append(self.totalElapsedSeconds)
                    self.refreshUI()
                    StoneSoundService.shared.playClick()
                    self.phase = .playing
                    self.refreshSuggestionsIfNeeded()
                }
            } catch {
                DispatchQueue.main.async {
                    self.refreshUI()
                    self.phase = .playing
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }

    func undo() {
        guard phase == .playing else { return }
        territory = []
        let didUndoAiMove = engine.undo()
        var undoneMoves = didUndoAiMove ? 1 : 0
        if didUndoAiMove {
            if engine.undo() {
                undoneMoves += 1
            }
        }
        if undoneMoves > 0 {
            recordedMoves.removeLast(min(undoneMoves, recordedMoves.count))
            moveElapsedAtMove.removeLast(min(undoneMoves, moveElapsedAtMove.count))
        }
        refreshUI()
        moveAnalysisHistory.removeAll { $0.moveNumber > moveCount }
    }

    func countFinalScore() {
        guard phase == .playing || phase == .paused else { return }
        guard !isScoringFinalResult else { return }
        guard moveCount > 0 else {
            finalScoreText = "估目 暂无分析"
            territory = []
            return
        }

        isScoringFinalResult = true
        finalScoreText = nil

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }

            do {
                let rawScore = try self.engine.finalScore()
                let result = FinalScoreService.makeResult(from: rawScore)
                let ownershipData = self.getEngineOwnership()
                DispatchQueue.main.async {
                    self.finalScoreText = result.displayText
                    self.territory = ownershipData
                    self.isScoringFinalResult = false
                }
            } catch {
                DispatchQueue.main.async {
                    self.finalScoreText = nil
                    self.errorMessage = error.localizedDescription
                    self.isScoringFinalResult = false
                }
            }
        }
    }

    private func getEngineOwnership() -> [[Bool?]] {
        let raw = engine.getOwnership()
        guard raw.count == 361 else { return [] }

        var grid = Array(repeating: Array(repeating: Optional<Bool>.none, count: 19), count: 19)
        for i in 0..<361 {
            let r = i / 19
            let c = i % 19
            let val = raw[i]
            if val < -0.15 {
                grid[r][c] = true   // black territory
            } else if val > 0.15 {
                grid[r][c] = false  // white territory
            }
        }
        return grid
    }

    func calculateTerritory() -> [[Bool?]] {
        let size = 19
        let boardSnapshot = board
        var territory = Array(repeating: Array(repeating: Optional<Bool>.none, count: size), count: size)
        var visited = Array(repeating: Array(repeating: false, count: size), count: size)

        func neighbors(of r: Int, _ c: Int) -> [(Int, Int)] {
            [(r-1,c), (r+1,c), (r,c-1), (r,c+1)].filter { $0.0 >= 0 && $0.0 < size && $0.1 >= 0 && $0.1 < size }
        }

        for r in 0..<size {
            for c in 0..<size {
                guard boardSnapshot[r][c] == nil, !visited[r][c] else { continue }

                var region: [(Int, Int)] = []
                var queue = [(r, c)]
                visited[r][c] = true
                var borderingColors = Set<String>()

                while !queue.isEmpty {
                    let (cr, cc) = queue.removeFirst()
                    region.append((cr, cc))
                    for (nr, nc) in neighbors(of: cr, cc) {
                        if let isBlack = boardSnapshot[nr][nc] {
                            borderingColors.insert(isBlack ? "b" : "w")
                        } else if !visited[nr][nc] {
                            visited[nr][nc] = true
                            queue.append((nr, nc))
                        }
                    }
                }

                let owner: Bool? = switch borderingColors.count {
                case 1: borderingColors.first == "b" ? true : false
                default: nil
                }

                for (pr, pc) in region {
                    territory[pr][pc] = owner
                }
            }
        }

        return territory
    }

    func clearTerritory() {
        territory = []
        finalScoreText = nil
    }

    func setHandicapCount(_ count: Int) {
        guard canChooseHandicap else { return }
        let normalizedCount = count == 0 ? 0 : count.clamped(to: 2...9)
        if normalizedCount > 0 {
            humanColor = "b"
        }
        AppState.shared.handicapCount = normalizedCount
        handicapCount = normalizedCount
        territory = []

        guard isEngineRunning else {
            resetLocalBoardState()
            return
        }

        do {
            try engine.setHandicap(normalizedCount)
            recordedMoves = []
            restoredPausedSession = nil
            moveElapsedAtMove = []
            accumulatedElapsed = 0
            gameStartTime = Date()
            refreshUI()
            refreshSuggestionsIfNeeded()
            if phase == .playing && shouldRequestOpeningMove {
                requestOpeningMove()
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func setShowingSuggestions(_ isShowing: Bool) {
        isShowingSuggestions = isShowing
        engineSettingsQueue.async { [weak self, engine] in
            engine.setSuggestionsEnabled(isShowing)
            guard isShowing else { return }
            let suggestions = engine.refreshMoveSuggestions()
            DispatchQueue.main.async {
                guard self?.isShowingSuggestions == true else { return }
                self?.moveSuggestions = suggestions
            }
        }
        if !isShowing {
            moveSuggestions = []
        }
    }

    private func refreshSuggestionsIfNeeded() {
        guard isShowingSuggestions else { return }
        engineSettingsQueue.async { [weak self, engine] in
            let suggestions = engine.refreshMoveSuggestions()
            DispatchQueue.main.async {
                guard self?.isShowingSuggestions == true else { return }
                self?.moveSuggestions = suggestions
            }
        }
    }

    var canChooseHumanColor: Bool {
        handicapCount == 0 && moveCount == 0 && (phase == .idle || phase == .connecting || phase == .playing || phase == .paused)
    }

    var canChooseHandicap: Bool {
        moveCount == 0 && (phase == .idle || phase == .connecting || phase == .playing || phase == .paused)
    }

    func setHumanColor(_ color: String) {
        guard canChooseHumanColor else { return }
        humanColor = color
    }

    func newGame() {
        clearSavedSession()
        recordedMoves = []
        restoredPausedSession = nil
        clearTerritory()
        resetLocalBoardState()
        if isEngineRunning {
            do {
                try engine.reset()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
        phase = .idle
    }

    func resetGame() {
        let selectedHumanColor = handicapCount > 0 ? "b" : humanColor
        clearSavedSession()
        do {
            if isEngineRunning {
                try engine.reset()
                try engine.setHandicap(handicapCount)
            }
            humanColor = selectedHumanColor
            recordedMoves = []
            restoredPausedSession = nil
            resetLocalBoardState()
            if isEngineRunning {
                refreshUI()
            }
            phase = isEngineRunning ? .playing : .idle
            if isEngineRunning {
                startDisplayTimer()
                if shouldRequestOpeningMove {
                    requestOpeningMove()
                }
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func saveGame() -> String? {
        let activeMoves = isReviewMode ? Array(recordedMoves.prefix(replayTargetMove)) : recordedMoves
        guard !activeMoves.isEmpty else { return nil }

        let movesWithAnalysis: [SavedMoveWithAnalysis] = activeMoves.enumerated().map { idx, move in
            let snapshot = moveAnalysisHistory.first { $0.moveNumber > idx }
            let elapsed = idx < moveElapsedAtMove.count ? moveElapsedAtMove[idx] : 0
            return SavedMoveWithAnalysis(
                color: move.color,
                vertex: move.vertex,
                winrateBlack: snapshot?.winrateBlack ?? winrateBlack,
                leadBlack: snapshot?.leadBlack ?? leadBlack,
                elapsedAtMove: elapsed
            )
        }

        let info = GameInfo(
            boardSize: 19,
            komi: 7.5,
            handicap: handicapCount,
            humanColor: humanColor,
            engineModel: modelName,
            savedAt: Date()
        )

        let file = SavedGameFile(
            gameInfo: info,
            finalScore: finalScoreText,
            totalElapsedSeconds: totalElapsedSeconds,
            moves: movesWithAnalysis
        )

        do {
            try GameStore.shared.save(file)
            return file.gameInfo.savedAt.ISO8601Format().replacingOccurrences(of: ":", with: "-")
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    func loadGame(id: String) {
        guard let file = try? GameStore.shared.load(id: id) else {
            errorMessage = "棋谱文件加载失败"
            return
        }

        endGame()
        resetLocalBoardState()

        let appState = AppState.shared
        handicapCount = file.gameInfo.handicap
        humanColor = file.gameInfo.humanColor
        AppState.shared.handicapCount = handicapCount

        phase = .connecting

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }

            guard self.engine.create() else {
                DispatchQueue.main.async { self.errorMessage = "引擎创建失败"; self.phase = .idle }
                return
            }

            do {
                try self.engine.start(
                    binaryPath: appState.kataGoBinaryPath,
                    configPath: appState.kataGoConfigPath,
                    modelPath: appState.kataGoModelPath,
                    boardSize: 19,
                    timeout: 120.0
                )
                try self.engine.setHandicap(file.gameInfo.handicap)

                for move in file.moves {
                    try self.engine.play(color: move.color, vertex: move.vertex)
                }

                let savedMoves = file.moves.map { SavedMove(color: $0.color, vertex: $0.vertex) }

                DispatchQueue.main.async {
                    self.isEngineRunning = true
                    self.recordedMoves = savedMoves
                    self.moveElapsedAtMove = file.moves.map(\.elapsedAtMove)
                    self.moveAnalysisHistory = self.buildAnalysisHistory(from: file.moves)
                    self.accumulatedElapsed = file.totalElapsedSeconds
                    self.finalScoreText = file.finalScore
                    self.refreshUI()
                    self.isReviewMode = true
                    self.replayTargetMove = savedMoves.count
                    self.reviewRecordedMovesSnapshot = savedMoves
                    self.reviewMoveElapsedSnapshot = self.moveElapsedAtMove
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

    private func buildAnalysisHistory(from moves: [SavedMoveWithAnalysis]) -> [MoveAnalysisSnapshot] {
        var history: [MoveAnalysisSnapshot] = []
        var lastWinrate = 0.5
        var lastLead = 0.0
        for (idx, move) in moves.enumerated() {
            if move.winrateBlack != lastWinrate || move.leadBlack != lastLead || idx == moves.count - 1 {
                history.append(MoveAnalysisSnapshot(
                    moveNumber: idx + 1,
                    winrateBlack: move.winrateBlack,
                    leadBlack: move.leadBlack,
                    evaluationAccuracy: 1.0,
                    currentPlayer: move.color == "b" ? "w" : "b"
                ))
                lastWinrate = move.winrateBlack
                lastLead = move.leadBlack
            }
        }
        return history
    }

    private var jumpGeneration = 0

    func jumpToMove(_ target: Int) {
        guard target >= 0, target <= recordedMoves.count else { return }
        guard isReviewMode || phase == .playing || phase == .paused else { return }
        territory = []

        if !isReviewMode {
            isReviewMode = true
            reviewRecordedMovesSnapshot = recordedMoves
            reviewMoveElapsedSnapshot = moveElapsedAtMove
        }

        replayTargetMove = target
        jumpGeneration &+= 1
        let generation = jumpGeneration

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }

            do {
                guard self.jumpGeneration == generation else { return }
                try self.engine.reset()
                guard self.jumpGeneration == generation else { return }
                try self.engine.setHandicap(self.handicapCount)

                for i in 0..<target {
                    guard self.jumpGeneration == generation else { return }
                    let move = self.recordedMoves[i]
                    try self.engine.play(color: move.color, vertex: move.vertex)
                }

                DispatchQueue.main.async {
                    guard self.jumpGeneration == generation else { return }
                    self.refreshUI()
                    self.refreshSuggestionsIfNeeded()
                    if let snapshot = self.moveAnalysisHistory.last(where: { $0.moveNumber <= target }) {
                        self.winrateBlack = snapshot.winrateBlack
                        self.leadBlack = snapshot.leadBlack
                        self.evaluationAccuracy = snapshot.evaluationAccuracy
                    } else {
                        self.winrateBlack = 0.5
                        self.leadBlack = 0.0
                        self.evaluationAccuracy = 0.0
                    }
                }
            } catch {
                DispatchQueue.main.async {
                    guard self.jumpGeneration == generation else { return }
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }

    func stepBackward() {
        guard isReviewMode, replayTargetMove > 0 else { return }
        jumpToMove(replayTargetMove - 1)
    }

    func stepForward() {
        guard isReviewMode, replayTargetMove < recordedMoves.count else { return }
        jumpToMove(replayTargetMove + 1)
    }

    func resetReviewPosition() {
        guard isReviewMode else { return }
        recordedMoves = reviewRecordedMovesSnapshot
        moveElapsedAtMove = reviewMoveElapsedSnapshot
        jumpToMove(replayTargetMove)
    }

    func continueFromReview() {
        guard isReviewMode else { return }
        recordedMoves = Array(recordedMoves.prefix(replayTargetMove))
        moveElapsedAtMove = Array(moveElapsedAtMove.prefix(replayTargetMove))
        moveAnalysisHistory.removeAll { $0.moveNumber > replayTargetMove }
        isReviewMode = false
        replayTargetMove = 0
        reviewRecordedMovesSnapshot = []
        reviewMoveElapsedSnapshot = []
        phase = .playing
        startDisplayTimer()
        if shouldRequestOpeningMove {
            requestOpeningMove()
        }
    }

    func exitReviewMode() {
        guard isReviewMode else { return }
        isReviewMode = false
        replayTargetMove = 0
        reviewRecordedMovesSnapshot = []
        reviewMoveElapsedSnapshot = []

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }

            do {
                try self.engine.reset()
                try self.engine.setHandicap(self.handicapCount)

                for move in self.recordedMoves {
                    try self.engine.play(color: move.color, vertex: move.vertex)
                }

                DispatchQueue.main.async {
                    self.refreshUI()
                }
            } catch {
                DispatchQueue.main.async {
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }

    var maxThinkSteps: Int {
        60
    }

    var completedThinkSteps: Int {
        Int((evaluationAccuracy * Double(maxThinkSteps)).rounded()).clamped(to: 0...maxThinkSteps)
    }

    var handicapLabel: String {
        handicapCount == 0 ? "不让子" : "让 \(handicapCount) 子"
    }

    private var shouldRequestOpeningMove: Bool {
        moveCount == 0 && currentPlayer != humanColor
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
        boardVersion &+= 1
        currentPlayer = frame.currentPlayer
        capturesBlack = frame.capturesBlack
        capturesWhite = frame.capturesWhite
        lastMove = frame.lastMove

        if let analysis = engine.getAnalysis() {
            winrateBlack = analysis.winrateBlack
            leadBlack = analysis.leadBlack.isFinite ? analysis.leadBlack : 0.0
            evaluationAccuracy = analysis.evaluationAccuracy
            moveCount = analysis.moveCount
            currentPlayer = analysis.currentPlayer
            capturesBlack = analysis.capturesBlack
            capturesWhite = analysis.capturesWhite
        }
        moveSuggestions = engine.getMoveSuggestions()

        if moveCount > 0, moveAnalysisHistory.last?.moveNumber != moveCount {
            moveAnalysisHistory.append(MoveAnalysisSnapshot(
                moveNumber: moveCount,
                winrateBlack: winrateBlack,
                leadBlack: leadBlack,
                evaluationAccuracy: evaluationAccuracy,
                currentPlayer: currentPlayer
            ))
        }
    }

    private func requestOpeningMove() {
        let aiColor = currentPlayer
        guard phase == .playing, moveCount == 0, aiColor != humanColor else { return }
        phase = .waiting
        territory = []

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }

            do {
                let thinkStart = Date()
                let aiMove = try self.engine.genmove(color: aiColor)
                let thinkDuration = Date().timeIntervalSince(thinkStart)

                DispatchQueue.main.async {
                    if aiMove.vertex.uppercased() != "RESIGN" {
                        self.recordedMoves.append(SavedMove(color: aiColor, vertex: aiMove.vertex))
                    }
                    self.lastThinkDuration = thinkDuration
                    self.moveElapsedAtMove.append(self.totalElapsedSeconds)
                    self.refreshUI()
                    StoneSoundService.shared.playClick()
                    self.phase = .playing
                    self.refreshSuggestionsIfNeeded()
                }
            } catch {
                DispatchQueue.main.async {
                    self.phase = .playing
                    self.errorMessage = error.localizedDescription
                }
            }
        }
    }

    private func vertexFromPoint(col: Int, row: Int) -> String {
        let column = columns[columns.index(columns.startIndex, offsetBy: col)]
        return "\(column)\(19 - row)"
    }

    private func replay(_ moves: [SavedMove]) throws {
        for move in moves {
            try engine.play(color: move.color, vertex: move.vertex)
        }
    }

    private func resetLocalBoardState() {
        board = Array(repeating: Array(repeating: nil, count: 19), count: 19)
        moveLabels = []
        moveCount = 0
        currentPlayer = "b"
        winrateBlack = 0.5
        leadBlack = 0.0
        evaluationAccuracy = 0.0
        capturesBlack = 0
        capturesWhite = 0
        lastMove = nil
        lastThinkDuration = nil
        finalScoreText = nil
        isScoringFinalResult = false
        showsMoveLabelsOnMainBoard = false
        showsCoordinatesOnMainBoard = false
        territory = []
        moveSuggestions = []
        isShowingSuggestions = false
        boardVersion &+= 1
        gameStartTime = nil
        accumulatedElapsed = 0
        moveElapsedAtMove = []
        moveAnalysisHistory = []
        isReviewMode = false
        replayTargetMove = 0
        stopDisplayTimer()
    }

    private func startDisplayTimer() {
        stopDisplayTimer()
        if gameStartTime == nil {
            gameStartTime = Date()
        }
        displayTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                self?.displayTick &+= 1
            }
        }
    }

    private func stopDisplayTimer() {
        displayTimer?.invalidate()
        displayTimer = nil
    }

    private func accumulateElapsedAndPauseTimer() {
        if let start = gameStartTime {
            accumulatedElapsed += Date().timeIntervalSince(start)
            gameStartTime = nil
        }
        stopDisplayTimer()
    }

    private func savePausedSession() {
        let stones = board.enumerated().flatMap { row, values in
            values.enumerated().compactMap { col, stone -> SavedStone? in
                guard let stone else { return nil }
                return SavedStone(col: col, row: row, color: stone ? "b" : "w")
            }
        }
        let labels = moveLabels.map { SavedMoveLabel(col: $0.col, row: $0.row, moveNumber: $0.moveNumber) }
        let session = SavedGameSession(
            humanColor: humanColor,
            currentPlayer: currentPlayer,
            moveCount: moveCount,
            winrateBlack: winrateBlack,
            leadBlack: leadBlack,
            evaluationAccuracy: evaluationAccuracy,
            capturesBlack: capturesBlack,
            capturesWhite: capturesWhite,
            lastMove: lastMove.map { SavedPoint(col: $0.col, row: $0.row) },
            lastThinkDuration: lastThinkDuration,
            finalScoreText: finalScoreText,
            showsMoveLabelsOnMainBoard: showsMoveLabelsOnMainBoard,
            showsCoordinatesOnMainBoard: showsCoordinatesOnMainBoard,
            handicapCount: handicapCount,
            modelName: modelName,
            accumulatedElapsedSeconds: totalElapsedSeconds,
            moveElapsedAtMove: moveElapsedAtMove,
            moves: recordedMoves,
            stones: stones,
            moveLabels: labels
        )
        do {
            let data = try JSONEncoder().encode(session)
            UserDefaults.standard.set(data, forKey: Self.pausedSessionKey)
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    private func restorePausedSessionIfAvailable() {
        guard let data = UserDefaults.standard.data(forKey: Self.pausedSessionKey) else { return }
        do {
            let session = try JSONDecoder().decode(SavedGameSession.self, from: data)
            humanColor = session.humanColor
            currentPlayer = session.currentPlayer
            moveCount = session.moveCount
            winrateBlack = session.winrateBlack
            leadBlack = session.leadBlack
            evaluationAccuracy = session.evaluationAccuracy
            capturesBlack = session.capturesBlack
            capturesWhite = session.capturesWhite
            lastMove = session.lastMove.map { ($0.col, $0.row) }
            lastThinkDuration = session.lastThinkDuration
            finalScoreText = session.finalScoreText
            showsMoveLabelsOnMainBoard = session.showsMoveLabelsOnMainBoard
            showsCoordinatesOnMainBoard = session.showsCoordinatesOnMainBoard
            handicapCount = session.handicapCount
            modelName = session.modelName
            accumulatedElapsed = session.accumulatedElapsedSeconds
            moveElapsedAtMove = session.moveElapsedAtMove
            recordedMoves = session.moves
            moveLabels = session.moveLabels.map { ($0.col, $0.row, $0.moveNumber) }

            var restoredBoard = Array(repeating: Array(repeating: Optional<Bool>.none, count: 19), count: 19)
            for stone in session.stones where stone.row >= 0 && stone.row < 19 && stone.col >= 0 && stone.col < 19 {
                restoredBoard[stone.row][stone.col] = stone.color == "b"
            }
            board = restoredBoard
            restoredPausedSession = session
            phase = .paused
        } catch {
            clearSavedSession()
        }
    }

    private func clearSavedSession() {
        UserDefaults.standard.removeObject(forKey: Self.pausedSessionKey)
    }

    private static let pausedSessionKey = "KataGoGo.pausedSession.v1"
}

private struct SavedGameSession: Codable {
    let humanColor: String
    let currentPlayer: String
    let moveCount: Int
    let winrateBlack: Double
    let leadBlack: Double
    let evaluationAccuracy: Double
    let capturesBlack: Int
    let capturesWhite: Int
    let lastMove: SavedPoint?
    let lastThinkDuration: TimeInterval?
    let finalScoreText: String?
    let showsMoveLabelsOnMainBoard: Bool
    let showsCoordinatesOnMainBoard: Bool
    let handicapCount: Int
    let modelName: String
    let accumulatedElapsedSeconds: TimeInterval
    let moveElapsedAtMove: [TimeInterval]
    let moves: [SavedMove]
    let stones: [SavedStone]
    let moveLabels: [SavedMoveLabel]
}

struct SavedMove: Codable {
    let color: String
    let vertex: String
}

private struct SavedPoint: Codable {
    let col: Int
    let row: Int
}

private struct SavedStone: Codable {
    let col: Int
    let row: Int
    let color: String
}

private struct SavedMoveLabel: Codable {
    let col: Int
    let row: Int
    let moveNumber: Int
}

struct SavedMoveWithAnalysis: Codable {
    let color: String
    let vertex: String
    let winrateBlack: Double
    let leadBlack: Double
    let elapsedAtMove: TimeInterval
}

struct MoveAnalysisSnapshot: Codable {
    let moveNumber: Int
    let winrateBlack: Double
    let leadBlack: Double
    let evaluationAccuracy: Double
    let currentPlayer: String
}

struct GameInfo: Codable {
    let boardSize: Int
    let komi: Double
    let handicap: Int
    let humanColor: String
    let engineModel: String
    let savedAt: Date
}

struct SavedGameFile: Codable {
    var version: Int = 1
    let gameInfo: GameInfo
    var finalScore: String?
    var totalElapsedSeconds: TimeInterval
    var moves: [SavedMoveWithAnalysis]
}

private extension Comparable {
    func clamped(to limits: ClosedRange<Self>) -> Self {
        min(max(self, limits.lowerBound), limits.upperBound)
    }
}
