use crate::analysis::AnalysisData;
use crate::analysis_events::{self, FfiAnalysisCallback};
use crate::auto_review::AutoReview;
use crate::engine_adapter::{EngineAdapter, EngineConfig};
use crate::game_state::{Color, GameState, MoveRecord, Vertex};
use crate::gtp_client::GtpClient;
use crate::move_history::MoveHistory;
use crate::play_tree::{AiSnapshot, BoardPoint, GoPlayTree, PlayMove};
use crate::render_frame::{
    BoardRenderer, CRenderFrameView, CReviewedMove, MoveLabel, MoveSuggestion, RenderFrameBuffer,
    StoneRender,
};
use once_cell::sync::Lazy;
use std::sync::Mutex;

// ── Global engine instance ─────────────────────────────────

struct GoCore {
    game_state: GameState,
    move_history: MoveHistory,
    play_tree: GoPlayTree,
    engine: Box<dyn EngineAdapter + Send>,
    renderer: BoardRenderer,
    render_frame_buffer: Option<RenderFrameBuffer>,
    last_error: String,
    analysis: AnalysisData,
    suggestions_enabled: bool,
    setup_stones: Vec<String>,
    auto_review_result: Option<AutoReview>,
}

impl GoCore {
    fn new() -> Self {
        GoCore {
            game_state: GameState::new(19),
            move_history: MoveHistory::new(),
            play_tree: GoPlayTree::new(),
            engine: Box::new(GtpClient::new()),
            renderer: BoardRenderer::new(),
            render_frame_buffer: None,
            last_error: String::new(),
            analysis: AnalysisData::new(),
            suggestions_enabled: false,
            setup_stones: Vec::new(),
            auto_review_result: None,
        }
    }
}

static INSTANCE: Lazy<Mutex<Option<GoCore>>> = Lazy::new(|| Mutex::new(None));
static TOKIO_RUNTIME: Lazy<Mutex<Option<tokio::runtime::Runtime>>> = Lazy::new(|| Mutex::new(None));

fn tokio_handle() -> Result<tokio::runtime::Handle, String> {
    let mut guard = TOKIO_RUNTIME
        .lock()
        .map_err(|e| format!("tokio runtime lock: {e}"))?;
    if guard.is_none() {
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime create: {e}"))?;
        let handle = rt.handle().clone();
        *guard = Some(rt);
        Ok(handle)
    } else {
        Ok(guard.as_ref().unwrap().handle().clone())
    }
}

fn block_on<F: std::future::Future>(future: F) -> Result<F::Output, String> {
    Ok(tokio_handle()?.block_on(future))
}

fn handicap_vertices(count: i32, board_size: u8) -> Option<Vec<&'static str>> {
    if board_size != 19 {
        return if count == 0 { Some(Vec::new()) } else { None };
    }

    match count {
        0 => Some(Vec::new()),
        2 => Some(vec!["D4", "Q16"]),
        3 => Some(vec!["D4", "Q16", "D16"]),
        4 => Some(vec!["D4", "Q16", "D16", "Q4"]),
        5 => Some(vec!["D4", "Q16", "D16", "Q4", "K10"]),
        6 => Some(vec!["D4", "Q16", "D16", "Q4", "D10", "Q10"]),
        7 => Some(vec!["D4", "Q16", "D16", "Q4", "D10", "Q10", "K10"]),
        8 => Some(vec!["D4", "Q16", "D16", "Q4", "D10", "Q10", "K4", "K16"]),
        9 => Some(vec![
            "D4", "Q16", "D16", "Q4", "D10", "Q10", "K4", "K16", "K10",
        ]),
        _ => None,
    }
}

fn reset_game_state_with_setup(core: &mut GoCore) {
    let board_size = core.game_state.board_size();
    core.game_state = GameState::new(board_size);
    let setup_refs: Vec<&str> = core.setup_stones.iter().map(String::as_str).collect();
    let _ = core.game_state.set_setup_stones(Color::Black, &setup_refs);
}

fn play_move_from_record(record: &MoveRecord, board_size: u8) -> crate::error::Result<PlayMove> {
    let vertex = record.vertex.trim();
    let mut play_move = if vertex.eq_ignore_ascii_case("pass") {
        PlayMove::pass(record.color)
    } else {
        let point = Vertex::from_coord(vertex, board_size)?;
        PlayMove::stone(record.color, BoardPoint::new(point.col, point.row))
    };

    if let (Some(winrate), Some(lead)) = (record.winrate, record.lead) {
        play_move = play_move.with_ai_snapshot(AiSnapshot {
            winrate: winrate as f32,
            lead: lead as f32,
            visits: 0,
        });
    }

    Ok(play_move)
}

// ── FFI Helpers ────────────────────────────────────────────

fn set_error(msg: String) {
    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            core.last_error = msg;
        }
    }
}

fn set_error_str(s: &str) {
    set_error(s.to_string());
}

// ── FFI Exports ────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn go_core_create() -> i32 {
    match INSTANCE.lock() {
        Ok(mut inst) => {
            *inst = Some(GoCore::new());
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn go_core_destroy() -> i32 {
    let _ = analysis_events::clear_analysis_callback();
    match INSTANCE.lock() {
        Ok(mut inst) => {
            *inst = None;
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn go_core_set_analysis_callback(callback: Option<FfiAnalysisCallback>) -> i32 {
    analysis_events::set_analysis_callback(callback)
}

#[no_mangle]
pub extern "C" fn go_core_clear_analysis_callback() -> i32 {
    analysis_events::clear_analysis_callback()
}

#[no_mangle]
pub extern "C" fn go_core_emit_current_analysis_frame() -> i32 {
    let frame = {
        let inst = match INSTANCE.lock() {
            Ok(i) => i,
            Err(_) => return -1,
        };
        let core = match inst.as_ref() {
            Some(c) => c,
            None => return -1,
        };
        let frame = core
            .renderer
            .render(&core.game_state, core.move_history.all_records());
        RenderFrameBuffer::from_frame(frame, core.analysis.move_suggestions.clone())
    };

    analysis_events::dispatch_render_frame(frame);
    0
}

#[no_mangle]
/// # Safety
///
/// `binary_path`, `config_path`, and `model_path` must be valid pointers to
/// null-terminated C strings for the duration of this call, or null.
pub unsafe extern "C" fn go_core_start(
    binary_path: *const std::os::raw::c_char,
    config_path: *const std::os::raw::c_char,
    model_path: *const std::os::raw::c_char,
    board_size: i32,
    timeout_secs: f64,
) -> i32 {
    let bin = unsafe { cstr_to_string(binary_path) };
    let cfg = unsafe { cstr_to_string(config_path) };
    let mdl = unsafe { cstr_to_string(model_path) };

    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    match block_on(core.engine.start(EngineConfig {
        binary_path: bin,
        config_path: cfg,
        model_path: mdl,
        board_size: board_size as u8,
        timeout_secs,
    })) {
        Ok(Ok(_)) => {
            core.game_state = GameState::new(board_size as u8);
            core.move_history = MoveHistory::new();
            core.play_tree = GoPlayTree::new();
            core.analysis = AnalysisData::new();
            core.setup_stones.clear();
            0
        }
        Ok(Err(e)) => {
            set_error_str(&e.to_string());
            -1
        }
        Err(e) => {
            set_error_str(&e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn go_core_close() -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    match block_on(core.engine.close()) {
        Ok(Ok(_)) => 0,
        Ok(Err(e)) => {
            set_error_str(&e.to_string());
            -1
        }
        Err(e) => {
            set_error_str(&e);
            -1
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `color` and `vertex` must be valid pointers to null-terminated C strings
/// for the duration of this call, or null.
pub unsafe extern "C" fn go_core_play(
    color: *const std::os::raw::c_char,
    vertex: *const std::os::raw::c_char,
) -> i32 {
    let c = cstr_to_string(color);
    let v = cstr_to_string(vertex);

    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    match block_on(core.engine.play(&c, &v)) {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            return -1;
        }
        Err(e) => {
            core.last_error = e;
            return -1;
        }
    }

    let color_enum = match Color::parse(&c) {
        Ok(c) => c,
        Err(e) => {
            core.last_error = e.to_string();
            return -1;
        }
    };

    match core.game_state.record_move(color_enum, &v) {
        Ok(record) => {
            match play_move_from_record(&record, core.game_state.board_size()) {
                Ok(play_move) => {
                    if let Err(e) = core.play_tree.play(play_move) {
                        core.last_error = e.to_string();
                        return -1;
                    }
                }
                Err(e) => {
                    core.last_error = e.to_string();
                    return -1;
                }
            }
            core.move_history.push(record);
            0
        }
        Err(e) => {
            core.last_error = e.to_string();
            -1
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `color` must be a valid pointer to a null-terminated C string, or null.
/// `out_vertex`, when non-null, must point to writable memory of at least
/// `out_vertex_len` bytes. `out_winrate` and `out_lead`, when non-null, must
/// point to valid writable `f64` values.
pub unsafe extern "C" fn go_core_genmove(
    color: *const std::os::raw::c_char,
    out_vertex: *mut std::os::raw::c_char,
    out_vertex_len: i32,
    out_winrate: *mut f64,
    out_lead: *mut f64,
) -> i32 {
    let c = cstr_to_string(color);

    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    let board_size = core.game_state.board_size();
    match block_on(core.engine.genmove(&c, board_size)) {
        Ok(Ok(genmove)) => {
            let vertex = genmove.vertex;
            let winrate_black = genmove.winrate_black;
            let lead_black = genmove.lead_black;
            let evaluation_accuracy = genmove.evaluation_accuracy;
            let v_bytes = vertex.as_bytes();
            let copy_len = v_bytes.len().min(out_vertex_len as usize - 1);
            std::ptr::copy_nonoverlapping(v_bytes.as_ptr(), out_vertex as *mut u8, copy_len);
            *out_vertex.add(copy_len) = 0;

            let color_enum = Color::parse(&c).unwrap_or(Color::Black);
            if let Ok(mut record) = core.game_state.record_move(color_enum, &vertex) {
                record.winrate = winrate_black;
                record.lead = lead_black;
                record.evaluation_accuracy = evaluation_accuracy;
                if let Ok(play_move) = play_move_from_record(&record, core.game_state.board_size())
                {
                    let _ = core.play_tree.play(play_move);
                }
                core.move_history.push(record);
            }

            if !out_winrate.is_null() {
                *out_winrate = winrate_black.unwrap_or(-1.0);
            }
            if !out_lead.is_null() {
                *out_lead = lead_black.unwrap_or(f64::NAN);
            }

            core.analysis.winrate_black = winrate_black.unwrap_or(0.5);
            core.analysis.lead_black = lead_black.unwrap_or(0.0);
            core.analysis.evaluation_accuracy = evaluation_accuracy.unwrap_or(0.0);
            core.analysis.move_count = core.game_state.move_count();
            core.analysis.current_player = core.game_state.current_player().as_str().to_string();

            core.analysis.move_suggestions.clear();

            0
        }
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            -1
        }
        Err(e) => {
            core.last_error = e;
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn go_core_last_error() -> *const std::os::raw::c_char {
    static LAST_ERROR: Lazy<Mutex<Vec<u8>>> = Lazy::new(|| Mutex::new(Vec::new()));

    let msg = match INSTANCE.lock() {
        Ok(inst) => inst
            .as_ref()
            .map(|c| c.last_error.clone())
            .unwrap_or_default(),
        Err(_) => "lock failed".to_string(),
    };
    let c_string = std::ffi::CString::new(msg).unwrap_or_default();
    match LAST_ERROR.lock() {
        Ok(mut buf) => {
            *buf = c_string.into_bytes_with_nul();
            buf.as_ptr() as *const std::os::raw::c_char
        }
        Err(_) => std::ptr::null(),
    }
}

#[no_mangle]
/// # Safety
///
/// Output buffers must either be null or point to writable memory large enough
/// for the corresponding `out_max_*` length. `out_current_player`, when
/// non-null, must be writable for at least `out_current_player_len` bytes.
pub unsafe extern "C" fn go_core_get_render_frame(
    out_stones: *mut StoneRender,
    out_max_stones: i32,
    out_num_stones: *mut i32,
    out_move_labels: *mut MoveLabel,
    out_max_labels: i32,
    out_num_labels: *mut i32,
    out_board_size: *mut i32,
    out_last_move_col: *mut i32,
    out_last_move_row: *mut i32,
    out_move_count: *mut i32,
    out_current_player: *mut std::os::raw::c_char,
    out_current_player_len: i32,
) -> i32 {
    let inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_ref() {
        Some(c) => c,
        None => return -1,
    };

    let frame = core
        .renderer
        .render(&core.game_state, core.move_history.all_records());

    if !out_board_size.is_null() {
        *out_board_size = frame.board_size as i32;
    }
    if !out_move_count.is_null() {
        *out_move_count = frame.move_count as i32;
    }
    if !out_last_move_col.is_null() {
        *out_last_move_col = frame.last_move.map(|(c, _)| c as i32).unwrap_or(-1);
    }
    if !out_last_move_row.is_null() {
        *out_last_move_row = frame.last_move.map(|(_, r)| r as i32).unwrap_or(-1);
    }

    if !out_current_player.is_null() {
        let cp = frame.current_player.as_bytes();
        let copy_len = cp.len().min(out_current_player_len as usize - 1);
        std::ptr::copy_nonoverlapping(cp.as_ptr(), out_current_player as *mut u8, copy_len);
        *out_current_player.add(copy_len) = 0;
    }

    let num_stones = frame.stones.len().min(out_max_stones as usize);
    if !out_num_stones.is_null() {
        *out_num_stones = num_stones as i32;
    }
    for i in 0..num_stones {
        *out_stones.add(i) = frame.stones[i].clone();
    }

    let num_labels = frame.move_labels.len().min(out_max_labels as usize);
    if !out_num_labels.is_null() {
        *out_num_labels = num_labels as i32;
    }
    for i in 0..num_labels {
        *out_move_labels.add(i) = frame.move_labels[i].clone();
    }

    0
}

#[no_mangle]
pub extern "C" fn go_core_get_render_frame_view() -> *const CRenderFrameView {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return std::ptr::null(),
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return std::ptr::null(),
    };

    let frame = core
        .renderer
        .render(&core.game_state, core.move_history.all_records());
    core.render_frame_buffer = Some(RenderFrameBuffer::from_frame(
        frame,
        core.analysis.move_suggestions.clone(),
    ));

    core.render_frame_buffer
        .as_ref()
        .map(|buffer| buffer.view() as *const CRenderFrameView)
        .unwrap_or(std::ptr::null())
}

#[no_mangle]
/// # Safety
///
/// `out_path`, when non-null, must point to writable memory for at least
/// `out_max_path` `u32` values. Other output pointers, when non-null, must be
/// valid writable pointers to their declared scalar types.
pub unsafe extern "C" fn go_core_get_play_tree_cursor(
    out_path: *mut u32,
    out_max_path: i32,
    out_path_len: *mut i32,
    out_current_move_number: *mut u32,
    out_child_count: *mut i32,
    out_active_line_len: *mut i32,
) -> i32 {
    let inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_ref() {
        Some(c) => c,
        None => return -1,
    };

    let path = core.play_tree.current_path();
    if !out_path_len.is_null() {
        *out_path_len = path.len() as i32;
    }
    if !out_current_move_number.is_null() {
        *out_current_move_number = core.play_tree.current_node().move_number;
    }
    if !out_child_count.is_null() {
        *out_child_count = core.play_tree.current_node().children.len() as i32;
    }
    if !out_active_line_len.is_null() {
        *out_active_line_len = core.play_tree.active_line().len() as i32;
    }

    if !out_path.is_null() && out_max_path > 0 {
        let copy_len = path.len().min(out_max_path as usize);
        for (i, index) in path.iter().take(copy_len).enumerate() {
            *out_path.add(i) = *index as u32;
        }
    }

    0
}

#[no_mangle]
/// # Safety
///
/// Output pointers, when non-null, must be valid writable pointers to their
/// declared scalar types. `out_current_player`, when non-null, must be writable
/// for at least `out_current_player_len` bytes.
pub unsafe extern "C" fn go_core_get_analysis(
    out_winrate_black: *mut f64,
    out_lead_black: *mut f64,
    out_move_count: *mut i32,
    out_current_player: *mut std::os::raw::c_char,
    out_current_player_len: i32,
    out_captures_black: *mut i32,
    out_captures_white: *mut i32,
    out_evaluation_accuracy: *mut f64,
) -> i32 {
    let inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_ref() {
        Some(c) => c,
        None => return -1,
    };

    if !out_winrate_black.is_null() {
        *out_winrate_black = core.analysis.winrate_black;
    }
    if !out_lead_black.is_null() {
        *out_lead_black = core.analysis.lead_black;
    }
    if !out_move_count.is_null() {
        *out_move_count = core.game_state.move_count() as i32;
    }
    if !out_captures_black.is_null() {
        *out_captures_black = core.game_state.captures().0 as i32;
    }
    if !out_captures_white.is_null() {
        *out_captures_white = core.game_state.captures().1 as i32;
    }
    if !out_evaluation_accuracy.is_null() {
        *out_evaluation_accuracy = core.analysis.evaluation_accuracy;
    }
    if !out_current_player.is_null() {
        let cp = core.game_state.current_player().as_str().as_bytes();
        let copy_len = cp.len().min(out_current_player_len as usize - 1);
        std::ptr::copy_nonoverlapping(cp.as_ptr(), out_current_player as *mut u8, copy_len);
        *out_current_player.add(copy_len) = 0;
    }
    0
}

#[no_mangle]
/// # Safety
///
/// `out_suggestions`, when non-null, must point to writable memory for at
/// least `out_max` `MoveSuggestion` values. `out_num`, when non-null, must be a
/// valid writable pointer.
pub unsafe extern "C" fn go_core_get_move_suggestions(
    out_suggestions: *mut MoveSuggestion,
    out_max: i32,
    out_num: *mut i32,
) -> i32 {
    let inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_ref() {
        Some(c) => c,
        None => return -1,
    };

    let num = core.analysis.move_suggestions.len().min(out_max as usize);
    if !out_num.is_null() {
        *out_num = num as i32;
    }
    for i in 0..num {
        *out_suggestions.add(i) = core.analysis.move_suggestions[i].clone();
    }
    0
}

#[no_mangle]
pub extern "C" fn go_core_undo() -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    if core.move_history.current_index() == 0 {
        return 0;
    }

    if core.engine.is_running() {
        match block_on(core.engine.undo()) {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                core.last_error = e.to_string();
                return -1;
            }
            Err(e) => {
                core.last_error = e;
                return -1;
            }
        }
    }

    let _record = match core.move_history.undo() {
        Some(r) => r,
        None => return 0,
    };
    let _ = core.play_tree.undo();

    reset_game_state_with_setup(core);
    for rec in core.move_history.records_up_to_current() {
        core.game_state.record_move(rec.color, &rec.vertex).ok();
    }

    if let Some(last) = core.move_history.last() {
        core.analysis.winrate_black = last.winrate.unwrap_or(0.5);
        core.analysis.lead_black = last.lead.unwrap_or(0.0);
        core.analysis.evaluation_accuracy = last.evaluation_accuracy.unwrap_or(0.0);
        core.analysis.move_suggestions.clear();
    } else {
        core.analysis = AnalysisData::new();
        core.analysis.current_player = core.game_state.current_player().as_str().to_string();
    }

    1
}

#[no_mangle]
pub extern "C" fn go_core_reset() -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    core.game_state.reset();
    core.move_history.reset();
    core.play_tree = GoPlayTree::new();
    core.analysis = AnalysisData::new();
    core.setup_stones.clear();

    match block_on(core.engine.clear_board(Some(30.0))) {
        Ok(Ok(_)) => 0,
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            -1
        }
        Err(e) => {
            core.last_error = e;
            -1
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `out_score`, when non-null, must point to writable memory for at least
/// `out_score_len` bytes.
pub unsafe extern "C" fn go_core_final_score(
    out_score: *mut std::os::raw::c_char,
    out_score_len: i32,
) -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    match block_on(core.engine.final_score()) {
        Ok(Ok(score)) => {
            if !out_score.is_null() && out_score_len > 0 {
                let bytes = score.as_bytes();
                let copy_len = bytes.len().min(out_score_len as usize - 1);
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_score as *mut u8, copy_len);
                *out_score.add(copy_len) = 0;
            }
            0
        }
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            -1
        }
        Err(e) => {
            core.last_error = e;
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn go_core_set_level(_level: i32) -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn go_core_set_handicap(count: i32) -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    if core.move_history.current_index() != 0 {
        core.last_error = "handicap can only be changed before moves are played".to_string();
        return -1;
    }

    let Some(vertices) = handicap_vertices(count, core.game_state.board_size()) else {
        core.last_error = "handicap must be 0 or 2-9 on a 19x19 board".to_string();
        return -1;
    };

    if core.engine.is_running() {
        match block_on(core.engine.clear_board(Some(30.0))) {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                core.last_error = e.to_string();
                return -1;
            }
            Err(e) => {
                core.last_error = e;
                return -1;
            }
        }
        if count > 0 {
            let command = format!("fixed_handicap {count}");
            match block_on(core.engine.command(&command, Some(30.0))) {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    core.last_error = e.to_string();
                    return -1;
                }
                Err(e) => {
                    core.last_error = e;
                    return -1;
                }
            }
        }
    }

    if let Err(e) = core.game_state.set_setup_stones(Color::Black, &vertices) {
        core.last_error = e.to_string();
        return -1;
    }

    core.setup_stones = vertices
        .iter()
        .map(|vertex| (*vertex).to_string())
        .collect();
    core.move_history.reset();
    core.play_tree = GoPlayTree::new();
    core.analysis = AnalysisData::new();
    core.analysis.current_player = core.game_state.current_player().as_str().to_string();
    0
}

#[no_mangle]
pub extern "C" fn go_core_set_suggestions_enabled(enabled: i32) -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    core.suggestions_enabled = enabled != 0;
    if !core.suggestions_enabled {
        core.analysis.move_suggestions.clear();
    }
    0
}

#[no_mangle]
pub extern "C" fn go_core_refresh_move_suggestions() -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    if !core.suggestions_enabled {
        core.analysis.move_suggestions.clear();
        return 0;
    }

    let board_size = core.game_state.board_size();
    let color = core.game_state.current_player().as_str().to_string();
    match block_on(core.engine.quick_analyze(&color, board_size)) {
        Ok(Ok(suggestions)) => {
            core.analysis.move_suggestions = suggestions;
            0
        }
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            core.analysis.move_suggestions.clear();
            -1
        }
        Err(e) => {
            core.last_error = e;
            core.analysis.move_suggestions.clear();
            -1
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `out_ownership`, when non-null, must point to writable memory for at least
/// `out_max` `f64` values.
pub unsafe extern "C" fn go_core_get_ownership(out_ownership: *mut f64, out_max: i32) -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    let board_size = core.game_state.board_size();
    match block_on(core.engine.analyze_ownership(board_size)) {
        Ok(Ok(ownership)) => {
            let num = ownership.len().min(out_max as usize);
            for (i, value) in ownership.iter().take(num).enumerate() {
                *out_ownership.add(i) = *value;
            }
            num as i32
        }
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            -1
        }
        Err(e) => {
            core.last_error = e;
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn go_core_run_auto_review(num_visits: i32) -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    let board_size = core.game_state.board_size();
    let records = core.move_history.all_records().to_vec();

    if records.is_empty() {
        core.auto_review_result = Some(AutoReview::empty());
        return 0;
    }

    match block_on(AutoReview::run_scan(
        core.engine.as_mut(),
        &records,
        board_size,
        num_visits,
    )) {
        Ok(Ok(review)) => {
            core.auto_review_result = Some(review);
            0
        }
        Ok(Err(e)) => {
            core.last_error = e.to_string();
            -1
        }
        Err(e) => {
            core.last_error = e;
            -1
        }
    }
}

#[no_mangle]
/// # Safety
///
/// `out_moves`, when non-null, must point to writable memory for at least
/// `out_max` `CReviewedMove` values. `out_num`, when non-null, must be a valid
/// writable pointer.
pub unsafe extern "C" fn go_core_get_auto_review_moves(
    out_moves: *mut CReviewedMove,
    out_max: i32,
    out_num: *mut i32,
) -> i32 {
    let inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_ref() {
        Some(c) => c,
        None => return -1,
    };

    let review = match &core.auto_review_result {
        Some(r) => r,
        None => {
            if !out_num.is_null() {
                *out_num = 0;
            }
            return 0;
        }
    };

    let num = review.moves.len().min(out_max as usize);
    if !out_num.is_null() {
        *out_num = num as i32;
    }

    for (i, rm) in review.moves.iter().take(num).enumerate() {
        let mut vertex_buf = [0u8; 8];
        let v = rm.vertex.as_bytes();
        let copy_len = v.len().min(7);
        vertex_buf[..copy_len].copy_from_slice(&v[..copy_len]);

        let (ai_col, ai_row, has_ai) = match rm.ai_best_move {
            Some((col, row)) => (col, row, 1u8),
            None => (0, 0, 0),
        };

        let color_byte: u8 = if rm.color == "w" { 1 } else { 0 };
        let quality_byte: u8 = match rm.quality {
            crate::auto_review::MoveQuality::Good => 0,
            crate::auto_review::MoveQuality::BadMove => 1,
            crate::auto_review::MoveQuality::SlackMove => 2,
        };

        *out_moves.add(i) = CReviewedMove {
            move_number: rm.move_number,
            color: color_byte,
            quality: quality_byte,
            _pad1: [0; 2],
            vertex: vertex_buf,
            winrate_before: rm.winrate_before,
            winrate_after: rm.winrate_after,
            score_before: rm.score_before,
            score_after: rm.score_after,
            winrate_drop: rm.winrate_drop,
            score_drop: rm.score_drop,
            ai_best_col: ai_col,
            ai_best_row: ai_row,
            has_ai_best: has_ai,
            suggestions_count: rm.top_suggestions.len() as u8,
            _pad2: [0; 4],
        };
    }

    0
}

// ── Helpers ────────────────────────────────────────────────

/// # Safety: ptr must be a valid null-terminated C string
unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_adapter::MockEngineAdapter;
    use crate::play_tree::BoardPoint;
    use std::ffi::CString;
    use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    static FFI_TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    static CALLBACK_CALLS: AtomicUsize = AtomicUsize::new(0);
    static CALLBACK_SUGGESTIONS_LEN: AtomicUsize = AtomicUsize::new(0);
    static CALLBACK_FIRST_VISITS: AtomicU32 = AtomicU32::new(0);

    extern "C" fn test_analysis_callback(frame: *const CRenderFrameView) {
        CALLBACK_CALLS.fetch_add(1, Ordering::SeqCst);
        if frame.is_null() {
            return;
        }
        unsafe {
            CALLBACK_SUGGESTIONS_LEN.store((*frame).suggestions_len, Ordering::SeqCst);
            if !(*frame).suggestions.is_null() && (*frame).suggestions_len > 0 {
                CALLBACK_FIRST_VISITS.store((*(*frame).suggestions).visits, Ordering::SeqCst);
            }
        }
    }

    #[test]
    fn handicap_vertices_support_default_and_two_to_nine() {
        assert_eq!(handicap_vertices(0, 19).unwrap(), Vec::<&str>::new());
        assert_eq!(handicap_vertices(2, 19).unwrap(), vec!["D4", "Q16"]);
        assert_eq!(
            handicap_vertices(9, 19).unwrap(),
            vec!["D4", "Q16", "D16", "Q4", "D10", "Q10", "K4", "K16", "K10"]
        );
        assert!(handicap_vertices(1, 19).is_none());
        assert!(handicap_vertices(2, 9).is_none());
    }

    #[test]
    fn render_frame_view_export_returns_a_cached_zero_copy_view() {
        let _guard = FFI_TEST_LOCK.lock().unwrap();
        assert_eq!(go_core_create(), 0);

        {
            let mut inst = INSTANCE.lock().unwrap();
            let core = inst.as_mut().unwrap();
            core.game_state.record_move(Color::Black, "D4").unwrap();
            core.move_history.push(crate::game_state::MoveRecord {
                move_number: 1,
                color: Color::Black,
                vertex: "D4".to_string(),
                captured: Vec::new(),
                winrate: None,
                lead: None,
                evaluation_accuracy: None,
            });
            core.analysis.move_suggestions.push(MoveSuggestion {
                col: 10,
                row: 10,
                winrate: 0.57,
                lead: 2.25,
                visits: 512,
                order: 1,
            });
        }

        let ptr = go_core_get_render_frame_view();

        assert!(!ptr.is_null());
        unsafe {
            assert_eq!((*ptr).board_size, 19);
            assert_eq!((*ptr).stones_len, 1);
            assert_eq!((*ptr).move_labels_len, 1);
            assert_eq!((*ptr).suggestions_len, 1);
            assert_eq!((*(*ptr).stones).col, 3);
            assert_eq!((*(*ptr).suggestions).visits, 512);
        }

        assert_eq!(go_core_destroy(), 0);
    }

    #[test]
    fn ffi_play_and_undo_keep_play_tree_cursor_in_sync() {
        let _guard = FFI_TEST_LOCK.lock().unwrap();
        assert_eq!(go_core_create(), 0);
        {
            let mut inst = INSTANCE.lock().unwrap();
            let core = inst.as_mut().unwrap();
            core.engine = Box::new(MockEngineAdapter::new());
        }

        let binary = CString::new("mock").unwrap();
        let config = CString::new("mock.cfg").unwrap();
        let model = CString::new("mock.bin.gz").unwrap();
        assert_eq!(
            unsafe { go_core_start(binary.as_ptr(), config.as_ptr(), model.as_ptr(), 19, 1.0) },
            0
        );

        let black = CString::new("b").unwrap();
        let d4 = CString::new("D4").unwrap();
        assert_eq!(unsafe { go_core_play(black.as_ptr(), d4.as_ptr()) }, 0);

        {
            let inst = INSTANCE.lock().unwrap();
            let core = inst.as_ref().unwrap();
            assert_eq!(core.play_tree.current_node().move_number, 1);
            assert_eq!(
                core.play_tree.current_node().point,
                Some(BoardPoint::new(3, 15))
            );
            assert_eq!(core.play_tree.current_path(), &[0]);
        }

        assert_eq!(go_core_undo(), 1);

        {
            let inst = INSTANCE.lock().unwrap();
            let core = inst.as_ref().unwrap();
            assert_eq!(core.play_tree.current_node().move_number, 0);
            assert!(core.play_tree.current_path().is_empty());
            assert_eq!(core.play_tree.active_line().len(), 1);
        }

        assert_eq!(go_core_destroy(), 0);
    }

    #[test]
    fn ffi_can_report_play_tree_cursor_for_swift_bridge() {
        let _guard = FFI_TEST_LOCK.lock().unwrap();
        assert_eq!(go_core_create(), 0);
        {
            let mut inst = INSTANCE.lock().unwrap();
            let core = inst.as_mut().unwrap();
            core.engine = Box::new(MockEngineAdapter::new());
        }

        let binary = CString::new("mock").unwrap();
        let config = CString::new("mock.cfg").unwrap();
        let model = CString::new("mock.bin.gz").unwrap();
        assert_eq!(
            unsafe { go_core_start(binary.as_ptr(), config.as_ptr(), model.as_ptr(), 19, 1.0) },
            0
        );

        let black = CString::new("b").unwrap();
        let white = CString::new("w").unwrap();
        let d4 = CString::new("D4").unwrap();
        let q16 = CString::new("Q16").unwrap();
        assert_eq!(unsafe { go_core_play(black.as_ptr(), d4.as_ptr()) }, 0);
        assert_eq!(unsafe { go_core_play(white.as_ptr(), q16.as_ptr()) }, 0);

        let mut path = [0u32; 8];
        let mut path_len = 0;
        let mut move_number = 0;
        let mut child_count = 0;
        let mut active_line_len = 0;

        assert_eq!(
            unsafe {
                go_core_get_play_tree_cursor(
                    path.as_mut_ptr(),
                    path.len() as i32,
                    &mut path_len,
                    &mut move_number,
                    &mut child_count,
                    &mut active_line_len,
                )
            },
            0
        );

        assert_eq!(path_len, 2);
        assert_eq!(&path[..path_len as usize], &[0, 0]);
        assert_eq!(move_number, 2);
        assert_eq!(child_count, 0);
        assert_eq!(active_line_len, 2);

        assert_eq!(go_core_destroy(), 0);
    }

    #[test]
    fn ffi_analysis_callback_receives_current_render_frame_view() {
        let _guard = FFI_TEST_LOCK.lock().unwrap();
        CALLBACK_CALLS.store(0, Ordering::SeqCst);
        CALLBACK_SUGGESTIONS_LEN.store(0, Ordering::SeqCst);
        CALLBACK_FIRST_VISITS.store(0, Ordering::SeqCst);

        assert_eq!(go_core_create(), 0);
        {
            let mut inst = INSTANCE.lock().unwrap();
            let core = inst.as_mut().unwrap();
            core.analysis.move_suggestions = vec![MoveSuggestion {
                col: 3,
                row: 15,
                winrate: 0.55,
                lead: 2.0,
                visits: 128,
                order: 0,
            }];
        }

        assert_eq!(
            go_core_set_analysis_callback(Some(test_analysis_callback)),
            0
        );
        assert_eq!(go_core_emit_current_analysis_frame(), 0);

        assert_eq!(CALLBACK_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(CALLBACK_SUGGESTIONS_LEN.load(Ordering::SeqCst), 1);
        assert_eq!(CALLBACK_FIRST_VISITS.load(Ordering::SeqCst), 128);

        assert_eq!(go_core_clear_analysis_callback(), 0);
        assert_eq!(go_core_destroy(), 0);
    }
}
