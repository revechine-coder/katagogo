use crate::analysis::AnalysisData;
use crate::game_state::{Color, GameState};
use crate::gtp_client::GtpClient;
use crate::move_history::MoveHistory;
use crate::render_frame::{BoardRenderer, MoveLabel, MoveSuggestion, StoneRender};
use once_cell::sync::Lazy;
use std::sync::Mutex;

// ── Global engine instance ─────────────────────────────────

struct GoCore {
    game_state: GameState,
    move_history: MoveHistory,
    gtp_client: GtpClient,
    renderer: BoardRenderer,
    last_error: String,
    analysis: AnalysisData,
    suggestions_enabled: bool,
    setup_stones: Vec<String>,
}

impl GoCore {
    fn new() -> Self {
        GoCore {
            game_state: GameState::new(19),
            move_history: MoveHistory::new(),
            gtp_client: GtpClient::new(),
            renderer: BoardRenderer::new(),
            last_error: String::new(),
            analysis: AnalysisData::new(),
            suggestions_enabled: false,
            setup_stones: Vec::new(),
        }
    }
}

static INSTANCE: Lazy<Mutex<Option<GoCore>>> = Lazy::new(|| Mutex::new(None));

static TOKIO_RUNTIME: Lazy<Mutex<Option<tokio::runtime::Runtime>>> =
    Lazy::new(|| Mutex::new(None));

fn tokio_handle() -> Result<tokio::runtime::Handle, String> {
    let mut guard = TOKIO_RUNTIME
        .lock()
        .map_err(|e| format!("tokio runtime lock: {e}"))?;
    if guard.is_none() {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("tokio runtime create: {e}"))?;
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
    match INSTANCE.lock() {
        Ok(mut inst) => {
            *inst = None;
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn go_core_start(
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

    match block_on(core.gtp_client.start(&bin, &cfg, &mdl, board_size as u8, timeout_secs)) {
        Ok(Ok(_)) => {
            core.game_state = GameState::new(board_size as u8);
            core.move_history = MoveHistory::new();
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

    match block_on(core.gtp_client.close()) {
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

    match block_on(core.gtp_client.play(&c, &v)) {
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

    let color_enum = match Color::from_str(&c) {
        Ok(c) => c,
        Err(e) => {
            core.last_error = e.to_string();
            return -1;
        }
    };

    match core.game_state.record_move(color_enum, &v) {
        Ok(record) => {
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
    match block_on(core.gtp_client.genmove(&c, board_size)) {
        Ok(Ok((vertex, winrate_black, lead_black, evaluation_accuracy, _ai_suggestions))) => {
            let v_bytes = vertex.as_bytes();
            let copy_len = v_bytes.len().min(out_vertex_len as usize - 1);
            std::ptr::copy_nonoverlapping(v_bytes.as_ptr(), out_vertex as *mut u8, copy_len);
            *out_vertex.add(copy_len) = 0;

            let color_enum = Color::from_str(&c).unwrap_or(Color::Black);
            if let Ok(mut record) = core.game_state.record_move(color_enum, &vertex) {
                record.winrate = winrate_black;
                record.lead = lead_black;
                record.evaluation_accuracy = evaluation_accuracy;
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

    if core.gtp_client.is_running() {
        match block_on(core.gtp_client.undo()) {
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
    core.analysis = AnalysisData::new();
    core.setup_stones.clear();

    match block_on(core.gtp_client.command("clear_board", Some(30.0))) {
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

    match block_on(core.gtp_client.final_score()) {
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

    if core.gtp_client.is_running() {
        match block_on(core.gtp_client.command("clear_board", Some(30.0))) {
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
            match block_on(core.gtp_client.command(&command, Some(30.0))) {
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

    core.setup_stones = vertices.iter().map(|vertex| (*vertex).to_string()).collect();
    core.move_history.reset();
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
    match block_on(core.gtp_client.quick_analyze(&color, board_size)) {
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
pub unsafe extern "C" fn go_core_get_ownership(
    out_ownership: *mut f64,
    out_max: i32,
) -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    let board_size = core.game_state.board_size();
    match block_on(core.gtp_client.analyze_ownership(board_size)) {
        Ok(Ok(ownership)) => {
            let num = ownership.len().min(out_max as usize);
            for i in 0..num {
                *out_ownership.add(i) = ownership[i];
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
}
