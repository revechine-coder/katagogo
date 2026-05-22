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

// ── C function declarations ─────────────────────────────────

let RUST_LIB = "go_core"

@_silgen_name("go_core_create")
func c_go_core_create() -> Int32

@_silgen_name("go_core_destroy")
func c_go_core_destroy() -> Int32

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
    _ out_lead: UnsafeMutablePointer<Double>
) -> Int32

@_silgen_name("go_core_undo")
func c_go_core_undo() -> Int32

@_silgen_name("go_core_reset")
func c_go_core_reset() -> Int32

@_silgen_name("go_core_set_level")
func c_go_core_set_level(_ level: Int32) -> Int32

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

@_silgen_name("go_core_get_analysis")
func c_go_core_get_analysis(
    _ out_winrate_black: UnsafeMutablePointer<Double>,
    _ out_lead: UnsafeMutablePointer<Double>,
    _ out_move_count: UnsafeMutablePointer<Int32>,
    _ out_current_player: UnsafeMutablePointer<CChar>,
    _ out_current_player_len: Int32,
    _ out_captures_black: UnsafeMutablePointer<Int32>,
    _ out_captures_white: UnsafeMutablePointer<Int32>
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

struct AnalysisData {
    let winrateBlack: Double
    let lead: Double
    let moveCount: Int
    let currentPlayer: String
    let capturesBlack: Int
    let capturesWhite: Int
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

// ── GoCoreBridge: main API ─────────────────────────────────

final class GoCoreBridge {

    private var initialized = false

    // ── Lifecycle ───────────────────────────────────────

    func create() -> Bool {
        let result = c_go_core_create()
        initialized = result == 0
        return initialized
    }

    func destroy() {
        if initialized {
            c_go_core_destroy()
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
            c_go_core_close()
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

    func genmove(color: String) throws -> (vertex: String, winrate: Double, lead: Double) {
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

    // ── Settings ────────────────────────────────────────

    func setLevel(_ level: Int) {
        c_go_core_set_level(Int32(level))
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

    // ── Analysis ────────────────────────────────────────

    func getAnalysis() -> AnalysisData? {
        var winrate: Double = 0
        var lead: Double = 0
        var moveCount: Int32 = 0
        var currentPlayer = [CChar](repeating: 0, count: 4)
        var capturesBlack: Int32 = 0
        var capturesWhite: Int32 = 0

        let result = c_go_core_get_analysis(
            &winrate, &lead, &moveCount,
            &currentPlayer, 4,
            &capturesBlack, &capturesWhite
        )

        guard result == 0 else { return nil }

        return AnalysisData(
            winrateBlack: winrate,
            lead: lead,
            moveCount: Int(moveCount),
            currentPlayer: String(cString: currentPlayer),
            capturesBlack: Int(capturesBlack),
            capturesWhite: Int(capturesWhite)
        )
    }

    // ── Error ───────────────────────────────────────────

    var lastError: String {
        return String(cString: c_go_core_last_error())
    }
}
