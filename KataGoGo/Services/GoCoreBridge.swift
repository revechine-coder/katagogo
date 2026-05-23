//
//  GoCoreBridge.swift
//  KataGoGo macOS Phase 1B
//
//  Swift-to-Rust FFI bridge for go_core C ABI.
//  All game logic calls go through this class.
//

import Foundation

// ── Rust C ABI struct mirrors (keep in sync with go_core.h) ──

struct StoneRender {
    let col: UInt8
    let row: UInt8
    let color: UInt8  // 0 = black, 1 = white
}

struct MoveLabel {
    let col: UInt8
    let row: UInt8
    let move_number: UInt32
    let is_last: UInt8  // 0 or 1
}

struct MoveSuggestion {
    let col: UInt8
    let row: UInt8
    let winrate: Double
    let lead: Double
    let visits: UInt32
    let order: UInt32
}

struct CReviewedMove {
    let move_number: UInt32
    let color: UInt8          // 0 = black, 1 = white
    let quality: UInt8        // 0 = good, 1 = bad_move, 2 = slack_move
    let _pad1: (UInt8, UInt8)
    let vertex: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)
    let winrate_before: Double
    let winrate_after: Double
    let score_before: Double
    let score_after: Double
    let winrate_drop: Double
    let score_drop: Double
    let ai_best_col: UInt8
    let ai_best_row: UInt8
    let has_ai_best: UInt8
    let suggestions_count: UInt8
    let _pad2: (UInt8, UInt8, UInt8, UInt8)
}

struct StarPoint {
    let col: UInt8
    let row: UInt8
}

struct CRenderFrameView {
    let board_size: UInt8
    let current_player: UInt8  // 0 = black, 1 = white
    let _padding: (UInt8, UInt8)
    let move_count: UInt32
    let captures_black: UInt32
    let captures_white: UInt32
    let last_move_col: Int16
    let last_move_row: Int16
    let stones: UnsafePointer<StoneRender>?
    let stones_len: UInt
    let move_labels: UnsafePointer<MoveLabel>?
    let move_labels_len: UInt
    let star_points: UnsafePointer<StarPoint>?
    let star_points_len: UInt
    let suggestions: UnsafePointer<MoveSuggestion>?
    let suggestions_len: UInt
}

typealias FfiAnalysisCallback = @convention(c) (UnsafeRawPointer?) -> Void

// ── C function declarations ─────────────────────────────────

let RUST_LIB = "go_core"

@_silgen_name("go_core_create")
func c_go_core_create() -> Int32

@_silgen_name("go_core_destroy")
func c_go_core_destroy() -> Int32

@_silgen_name("go_core_set_analysis_callback")
func c_go_core_set_analysis_callback(_ callback: FfiAnalysisCallback?) -> Int32

@_silgen_name("go_core_clear_analysis_callback")
func c_go_core_clear_analysis_callback() -> Int32

@_silgen_name("go_core_start")
func c_go_core_start(
    _ binary_path: UnsafePointer<CChar>,
    _ config_path: UnsafePointer<CChar>,
    _ model_path: UnsafePointer<CChar>,
    _ board_size: Int32,
    _ timeout_secs: Double
) -> Int32

@_silgen_name("go_core_close")
func c_go_core_close() -> Int32

@_silgen_name("go_core_play")
func c_go_core_play(
    _ color: UnsafePointer<CChar>,
    _ vertex: UnsafePointer<CChar>
) -> Int32

@_silgen_name("go_core_genmove")
func c_go_core_genmove(
    _ color: UnsafePointer<CChar>,
    _ out_vertex: UnsafeMutablePointer<CChar>,
    _ out_vertex_len: Int32,
    _ out_winrate: UnsafeMutablePointer<Double>,
    _ out_lead_black: UnsafeMutablePointer<Double>
) -> Int32

@_silgen_name("go_core_undo")
func c_go_core_undo() -> Int32

@_silgen_name("go_core_reset")
func c_go_core_reset() -> Int32

@_silgen_name("go_core_final_score")
func c_go_core_final_score(
    _ out_score: UnsafeMutablePointer<CChar>,
    _ out_score_len: Int32
) -> Int32

@_silgen_name("go_core_set_level")
func c_go_core_set_level(_ level: Int32) -> Int32

@_silgen_name("go_core_set_handicap")
func c_go_core_set_handicap(_ count: Int32) -> Int32
@_silgen_name("go_core_set_suggestions_enabled")
func c_go_core_set_suggestions_enabled(_ enabled: Int32) -> Int32

@_silgen_name("go_core_refresh_move_suggestions")
func c_go_core_refresh_move_suggestions() -> Int32

@_silgen_name("go_core_get_render_frame")
func c_go_core_get_render_frame(
    _ out_stones: UnsafeMutablePointer<StoneRender>,
    _ out_max_stones: Int32,
    _ out_num_stones: UnsafeMutablePointer<Int32>,
    _ out_move_labels: UnsafeMutablePointer<MoveLabel>,
    _ out_max_labels: Int32,
    _ out_num_labels: UnsafeMutablePointer<Int32>,
    _ out_board_size: UnsafeMutablePointer<Int32>,
    _ out_last_move_col: UnsafeMutablePointer<Int32>,
    _ out_last_move_row: UnsafeMutablePointer<Int32>,
    _ out_move_count: UnsafeMutablePointer<Int32>,
    _ out_current_player: UnsafeMutablePointer<CChar>,
    _ out_current_player_len: Int32
) -> Int32

@_silgen_name("go_core_get_render_frame_view")
func c_go_core_get_render_frame_view() -> UnsafePointer<CRenderFrameView>?

@_silgen_name("go_core_get_play_tree_cursor")
func c_go_core_get_play_tree_cursor(
    _ out_path: UnsafeMutablePointer<UInt32>,
    _ out_max_path: Int32,
    _ out_path_len: UnsafeMutablePointer<Int32>,
    _ out_current_move_number: UnsafeMutablePointer<UInt32>,
    _ out_child_count: UnsafeMutablePointer<Int32>,
    _ out_active_line_len: UnsafeMutablePointer<Int32>
) -> Int32

@_silgen_name("go_core_get_analysis")
func c_go_core_get_analysis(
    _ out_winrate_black: UnsafeMutablePointer<Double>,
    _ out_lead: UnsafeMutablePointer<Double>,
    _ out_move_count: UnsafeMutablePointer<Int32>,
    _ out_current_player: UnsafeMutablePointer<CChar>,
    _ out_current_player_len: Int32,
    _ out_captures_black: UnsafeMutablePointer<Int32>,
    _ out_captures_white: UnsafeMutablePointer<Int32>,
    _ out_evaluation_accuracy: UnsafeMutablePointer<Double>
) -> Int32

@_silgen_name("go_core_get_move_suggestions")
func c_go_core_get_move_suggestions(
    _ out_suggestions: UnsafeMutablePointer<MoveSuggestion>,
    _ out_max: Int32,
    _ out_num: UnsafeMutablePointer<Int32>
) -> Int32

@_silgen_name("go_core_get_ownership")
func c_go_core_get_ownership(
    _ out_ownership: UnsafeMutablePointer<Double>,
    _ out_max: Int32
) -> Int32

@_silgen_name("go_core_run_auto_review")
nonisolated func c_go_core_run_auto_review(_ num_visits: Int32) -> Int32

@_silgen_name("go_core_get_auto_review_moves")
nonisolated func c_go_core_get_auto_review_moves(
    _ out_moves: UnsafeMutablePointer<CReviewedMove>,
    _ out_max: Int32,
    _ out_num: UnsafeMutablePointer<Int32>
) -> Int32

@_silgen_name("go_core_last_error")
func c_go_core_last_error() -> UnsafePointer<CChar>

// ── Swift-friendly render frame ─────────────────────────────

struct RenderFrame {
    let boardSize: Int
    let stones: [(col: Int, row: Int, isBlack: Bool)]
    let moveLabels: [(col: Int, row: Int, moveNumber: Int)]
    let lastMove: (col: Int, row: Int)?
    let moveCount: Int
    let currentPlayer: String
    let capturesBlack: Int
    let capturesWhite: Int
}

struct RenderFrameViewSnapshot {
    let boardSize: Int
    let stones: [(col: Int, row: Int, isBlack: Bool)]
    let moveLabels: [(col: Int, row: Int, moveNumber: Int)]
    let starPoints: [(col: Int, row: Int)]
    let suggestions: [(col: Int, row: Int, winrate: Double, lead: Double, visits: Int, order: Int)]
    let lastMove: (col: Int, row: Int)?
    let moveCount: Int
    let currentPlayer: String
    let capturesBlack: Int
    let capturesWhite: Int
}

struct PlayTreeCursor {
    let path: [Int]
    let currentMoveNumber: Int
    let childCount: Int
    let activeLineLength: Int
}

struct AnalysisData {
    let winrateBlack: Double
    let leadBlack: Double
    let evaluationAccuracy: Double
    let moveCount: Int
    let currentPlayer: String
    let capturesBlack: Int
    let capturesWhite: Int
}

enum ReviewedMoveQuality: String {
    case good = "好手"
    case badMove = "恶手"
    case slackMove = "缓着"

    nonisolated static func from(_ raw: UInt8) -> ReviewedMoveQuality {
        switch raw {
        case 1: return .badMove
        case 2: return .slackMove
        default: return .good
        }
    }
}

struct ReviewedMoveItem: Identifiable {
    let id: Int
    let moveNumber: Int
    let color: String          // "b" or "w"
    let vertex: String
    let winrateBefore: Double
    let winrateAfter: Double
    let scoreBefore: Double
    let scoreAfter: Double
    let winrateDrop: Double
    let scoreDrop: Double
    let quality: ReviewedMoveQuality
    let aiBestMove: (col: Int, row: Int)?
    let suggestionsCount: Int

    nonisolated init(from m: CReviewedMove) {
        self.id = Int(m.move_number)
        self.moveNumber = Int(m.move_number)
        self.color = m.color == 1 ? "w" : "b"
        self.vertex = String(cString: withUnsafePointer(to: m.vertex) {
            $0.withMemoryRebound(to: CChar.self, capacity: 8) { $0 }
        })
        self.winrateBefore = m.winrate_before
        self.winrateAfter = m.winrate_after
        self.scoreBefore = m.score_before
        self.scoreAfter = m.score_after
        self.winrateDrop = m.winrate_drop
        self.scoreDrop = m.score_drop
        self.quality = ReviewedMoveQuality.from(m.quality)
        self.aiBestMove = m.has_ai_best == 1
            ? (col: Int(m.ai_best_col), row: Int(m.ai_best_row))
            : nil
        self.suggestionsCount = Int(m.suggestions_count)
    }
}

struct AutoReviewReport {
    let moves: [ReviewedMoveItem]
    let totalMoves: Int
    let badMoveCount: Int
    let slackMoveCount: Int

    var goodMoveCount: Int { totalMoves - badMoveCount - slackMoveCount }
    var hasIssues: Bool { badMoveCount > 0 || slackMoveCount > 0 }
}

// ── Error ───────────────────────────────────────────────────

enum GoCoreError: LocalizedError {
    case engineNotInitialized
    case commandFailed(String)

    var errorDescription: String? {
        switch self {
        case .engineNotInitialized: return "GoCore engine not initialized"
        case .commandFailed(let msg): return msg
        }
    }
}

private let analysisCallbackQueue = DispatchQueue(label: "winner.KataGoGo.analysisCallback", qos: .userInitiated)
private var analysisSnapshotHandler: ((RenderFrameViewSnapshot) -> Void)?

private let rustAnalysisCallback: FfiAnalysisCallback = { framePointer in
    guard let framePointer else { return }
    let typedPointer = framePointer.bindMemory(to: CRenderFrameView.self, capacity: 1)
    guard let snapshot = GoCoreBridge.snapshot(from: typedPointer) else { return }
    analysisCallbackQueue.async {
        analysisSnapshotHandler?(snapshot)
    }
}

// ── GoCoreBridge: main API ─────────────────────────────────

final class GoCoreBridge {

    private var initialized = false
    var onAnalysisFrame: ((RenderFrameViewSnapshot) -> Void)?

    // ── Lifecycle ───────────────────────────────────────

    func create() -> Bool {
        let result = c_go_core_create()
        initialized = result == 0
        return initialized
    }

    func destroy() {
        if initialized {
            unregisterAnalysisCallback()
            _ = c_go_core_destroy()
            initialized = false
        }
    }

    deinit {
        destroy()
    }

    // ── Start / Stop KataGo ─────────────────────────────

    func start(binaryPath: String, configPath: String, modelPath: String,
               boardSize: Int = 19, timeout: Double = 120.0) throws {
        guard initialized else { throw GoCoreError.engineNotInitialized }
        registerAnalysisCallback()

        let result = binaryPath.withCString { bin in
            configPath.withCString { cfg in
                modelPath.withCString { mdl in
                    c_go_core_start(bin, cfg, mdl, Int32(boardSize), timeout)
                }
            }
        }

        if result != 0 {
            throw GoCoreError.commandFailed(lastError)
        }
    }

    func close() {
        if initialized {
            unregisterAnalysisCallback()
            _ = c_go_core_close()
        }
    }

    private func registerAnalysisCallback() {
        analysisCallbackQueue.sync {
            analysisSnapshotHandler = { [weak self] snapshot in
                self?.onAnalysisFrame?(snapshot)
            }
        }
        _ = c_go_core_set_analysis_callback(rustAnalysisCallback)
    }

    private func unregisterAnalysisCallback() {
        _ = c_go_core_clear_analysis_callback()
        analysisCallbackQueue.sync {
            analysisSnapshotHandler = nil
        }
    }

    // ── Gameplay ────────────────────────────────────────

    func play(color: String, vertex: String) throws {
        let result = color.withCString { c in
            vertex.withCString { v in
                c_go_core_play(c, v)
            }
        }
        if result != 0 {
            throw GoCoreError.commandFailed(lastError)
        }
    }

    func genmove(color: String) throws -> (vertex: String, winrateBlack: Double, leadBlack: Double) {
        var outVertex = [CChar](repeating: 0, count: 32)
        var outWinrate: Double = 0
        var outLead: Double = 0

        let result = color.withCString { c in
            c_go_core_genmove(c, &outVertex, 32, &outWinrate, &outLead)
        }

        if result != 0 {
            throw GoCoreError.commandFailed(lastError)
        }

        let vertex = String(cString: outVertex)
        return (vertex, outWinrate, outLead)
    }

    func undo() -> Bool {
        return c_go_core_undo() > 0
    }

    func reset() throws {
        let result = c_go_core_reset()
        if result != 0 {
            throw GoCoreError.commandFailed(lastError)
        }
    }

    func finalScore() throws -> String {
        var outScore = [CChar](repeating: 0, count: 128)
        let result = c_go_core_final_score(&outScore, Int32(outScore.count))
        if result != 0 {
            throw GoCoreError.commandFailed(lastError)
        }
        return String(cString: outScore)
    }

    // ── Settings ────────────────────────────────────────

    func setHandicap(_ count: Int) throws {
        let result = c_go_core_set_handicap(Int32(count))
        if result != 0 {
            throw GoCoreError.commandFailed(lastError)
        }
    }

    func setSuggestionsEnabled(_ enabled: Bool) {
        _ = c_go_core_set_suggestions_enabled(enabled ? 1 : 0)
    }

    func refreshMoveSuggestions() -> [(col: Int, row: Int, winrate: Double, lead: Double, visits: Int, order: Int)] {
        let result = c_go_core_refresh_move_suggestions()
        guard result == 0 else { return [] }
        return getMoveSuggestions()
    }

    // ── Render Frame ────────────────────────────────────

    func getRenderFrame() -> RenderFrame? {
        let maxStones = 361
        let maxLabels = 361

        var stones = [StoneRender](repeating: StoneRender(col: 0, row: 0, color: 0), count: maxStones)
        var numStones: Int32 = 0
        var labels = [MoveLabel](repeating: MoveLabel(col: 0, row: 0, move_number: 0, is_last: 0), count: maxLabels)
        var numLabels: Int32 = 0
        var boardSize: Int32 = 0
        var lastCol: Int32 = -1
        var lastRow: Int32 = -1
        var moveCount: Int32 = 0
        var currentPlayer = [CChar](repeating: 0, count: 4)

        let result = c_go_core_get_render_frame(
            &stones, Int32(maxStones), &numStones,
            &labels, Int32(maxLabels), &numLabels,
            &boardSize, &lastCol, &lastRow, &moveCount,
            &currentPlayer, 4
        )

        guard result == 0 else { return nil }

        let stoneArray = (0..<Int(numStones)).map { i in
            (col: Int(stones[i].col), row: Int(stones[i].row), isBlack: stones[i].color == 0)
        }

        let labelArray = (0..<Int(numLabels)).map { i in
            (col: Int(labels[i].col), row: Int(labels[i].row), moveNumber: Int(labels[i].move_number))
        }

        return RenderFrame(
            boardSize: Int(boardSize),
            stones: stoneArray,
            moveLabels: labelArray,
            lastMove: lastCol >= 0 ? (Int(lastCol), Int(lastRow)) : nil,
            moveCount: Int(moveCount),
            currentPlayer: String(cString: currentPlayer),
            capturesBlack: 0,
            capturesWhite: 0
        )
    }

    func getRenderFrameViewSnapshot() -> RenderFrameViewSnapshot? {
        Self.snapshot(from: c_go_core_get_render_frame_view())
    }

    func getPlayTreeCursor(maxDepth: Int = 512) -> PlayTreeCursor? {
        var path = [UInt32](repeating: 0, count: maxDepth)
        var pathLen: Int32 = 0
        var currentMoveNumber: UInt32 = 0
        var childCount: Int32 = 0
        var activeLineLength: Int32 = 0

        let result = c_go_core_get_play_tree_cursor(
            &path,
            Int32(maxDepth),
            &pathLen,
            &currentMoveNumber,
            &childCount,
            &activeLineLength
        )
        guard result == 0 else { return nil }

        let copiedPath = path.prefix(Int(min(pathLen, Int32(maxDepth)))).map(Int.init)
        return PlayTreeCursor(
            path: copiedPath,
            currentMoveNumber: Int(currentMoveNumber),
            childCount: Int(childCount),
            activeLineLength: Int(activeLineLength)
        )
    }

    // ── Analysis ────────────────────────────────────────

    func getAnalysis() -> AnalysisData? {
        var winrateBlack: Double = 0
        var leadBlack: Double = 0
        var moveCount: Int32 = 0
        var currentPlayer = [CChar](repeating: 0, count: 4)
        var capturesBlack: Int32 = 0
        var capturesWhite: Int32 = 0
        var evaluationAccuracy: Double = 0

        let result = c_go_core_get_analysis(
            &winrateBlack, &leadBlack, &moveCount,
            &currentPlayer, 4,
            &capturesBlack, &capturesWhite,
            &evaluationAccuracy
        )

        guard result == 0 else { return nil }

        return AnalysisData(
            winrateBlack: winrateBlack,
            leadBlack: leadBlack,
            evaluationAccuracy: evaluationAccuracy,
            moveCount: Int(moveCount),
            currentPlayer: String(cString: currentPlayer),
            capturesBlack: Int(capturesBlack),
            capturesWhite: Int(capturesWhite)
        )
    }

    // ── Ownership ──────────────────────────────────────

    func getOwnership() -> [Double] {
        var ownership = [Double](repeating: 0, count: 361)
        let num = c_go_core_get_ownership(&ownership, 361)
        guard num > 0 else { return [] }
        return Array(ownership.prefix(Int(num)))
    }

    // ── Move Suggestions ────────────────────────────────

    func getMoveSuggestions() -> [(col: Int, row: Int, winrate: Double, lead: Double, visits: Int, order: Int)] {
        let maxSuggestions = 10
        var suggestions = [MoveSuggestion](repeating: MoveSuggestion(col: 0, row: 0, winrate: 0, lead: 0, visits: 0, order: 0), count: maxSuggestions)
        var num: Int32 = 0

        let result = c_go_core_get_move_suggestions(&suggestions, Int32(maxSuggestions), &num)
        guard result == 0 else { return [] }

        return (0..<Int(num)).map { i in
            (col: Int(suggestions[i].col),
             row: Int(suggestions[i].row),
             winrate: suggestions[i].winrate,
             lead: suggestions[i].lead,
             visits: Int(suggestions[i].visits),
             order: Int(suggestions[i].order))
        }
    }

    // ── Auto Review ────────────────────────────────────

    nonisolated func runAutoReview(visits: Int32) -> Bool {
        return c_go_core_run_auto_review(visits) == 0
    }

    nonisolated func getAutoReviewMoves() -> [ReviewedMoveItem] {
        let maxMoves = 400
        var moves = [CReviewedMove](repeating: CReviewedMove(
            move_number: 0, color: 0, quality: 0, _pad1: (0, 0),
            vertex: (0, 0, 0, 0, 0, 0, 0, 0),
            winrate_before: 0, winrate_after: 0,
            score_before: 0, score_after: 0,
            winrate_drop: 0, score_drop: 0,
            ai_best_col: 0, ai_best_row: 0,
            has_ai_best: 0, suggestions_count: 0,
            _pad2: (0, 0, 0, 0)
        ), count: maxMoves)
        var num: Int32 = 0

        let result = c_go_core_get_auto_review_moves(&moves, Int32(maxMoves), &num)
        guard result == 0 else { return [] }

        return (0..<Int(num)).compactMap { i -> ReviewedMoveItem? in
            let m = moves[i]
            return ReviewedMoveItem(from: m)
        }
    }

    // ── Error ───────────────────────────────────────────

    var lastError: String {
        return String(cString: c_go_core_last_error())
    }

    private func array<T>(from pointer: UnsafePointer<T>?, count: Int) -> [T] {
        guard let pointer, count > 0 else { return [] }
        return Array(UnsafeBufferPointer(start: pointer, count: count))
    }

    static func snapshot(from viewPointer: UnsafePointer<CRenderFrameView>?) -> RenderFrameViewSnapshot? {
        guard let viewPointer else { return nil }
        let view = viewPointer.pointee

        let stones = copyArray(from: view.stones, count: Int(view.stones_len)).map { stone in
            (col: Int(stone.col), row: Int(stone.row), isBlack: stone.color == 0)
        }
        let labels = copyArray(from: view.move_labels, count: Int(view.move_labels_len)).map { label in
            (col: Int(label.col), row: Int(label.row), moveNumber: Int(label.move_number))
        }
        let starPoints = copyArray(from: view.star_points, count: Int(view.star_points_len)).map { point in
            (col: Int(point.col), row: Int(point.row))
        }
        let suggestions = copyArray(from: view.suggestions, count: Int(view.suggestions_len)).map { suggestion in
            (col: Int(suggestion.col),
             row: Int(suggestion.row),
             winrate: suggestion.winrate,
             lead: suggestion.lead,
             visits: Int(suggestion.visits),
             order: Int(suggestion.order))
        }

        return RenderFrameViewSnapshot(
            boardSize: Int(view.board_size),
            stones: stones,
            moveLabels: labels,
            starPoints: starPoints,
            suggestions: suggestions,
            lastMove: view.last_move_col >= 0 ? (Int(view.last_move_col), Int(view.last_move_row)) : nil,
            moveCount: Int(view.move_count),
            currentPlayer: view.current_player == 0 ? "b" : "w",
            capturesBlack: Int(view.captures_black),
            capturesWhite: Int(view.captures_white)
        )
    }

    private static func copyArray<T>(from pointer: UnsafePointer<T>?, count: Int) -> [T] {
        guard let pointer, count > 0 else { return [] }
        return Array(UnsafeBufferPointer(start: pointer, count: count))
    }
}
