use crate::analysis::AnalysisData;
use crate::error::GoCoreError;
use crate::game_state::{Color, GameState, MoveRecord};
use crate::gtp_client::GtpClient;
use crate::move_history::MoveHistory;
use crate::render_frame::{BoardRenderer, MoveLabel, StoneRender};
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
        }
    }
}

static INSTANCE: Lazy<Mutex<Option<GoCore>>> = Lazy::new(|| Mutex::new(None));

// ── FFI Helpers ────────────────────────────────────────────

fn set_error(msg: String) {
    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            core.last_error = msg;
        }
    }
}

fn with_core<F, R>(f: F) -> Result<R, ()>
where
    F: FnOnce(&mut GoCore) -> Result<R, GoCoreError>,
{
    let mut inst = INSTANCE.lock().map_err(|_| ())?;
    let core = inst.as_mut().ok_or(())?;
    f(core).map_err(|e| {
        core.last_error = e.to_string();
    })
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

    match INSTANCE.lock() {
        Ok(mut inst) => {
            if let Some(ref mut core) = *inst {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(r) => r,
                    Err(e) => {
                        set_error_str(&format!("tokio runtime: {e}"));
                        return -1;
                    }
                };
                match rt.block_on(core.gtp_client.start(
                    &bin,
                    &cfg,
                    &mdl,
                    board_size as u8,
                    timeout_secs,
                )) {
                    Ok(_) => {
                        core.game_state = GameState::new(board_size as u8);
                        core.move_history = MoveHistory::new();
                        0
                    }
                    Err(e) => {
                        set_error_str(&e.to_string());
                        -1
                    }
                }
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn go_core_close() -> i32 {
    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    if let Some(ref mut core) = *inst {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(_) => return -1,
        };
        match rt.block_on(core.gtp_client.close()) {
            Ok(_) => 0,
            Err(e) => {
                set_error_str(&e.to_string());
                -1
            }
        }
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn go_core_play(color: *const std::os::raw::c_char, vertex: *const std::os::raw::c_char) -> i32 {
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

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    // Send play to KataGo
    if let Err(e) = rt.block_on(core.gtp_client.play(&c, &v)) {
        core.last_error = e.to_string();
        return -1;
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

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    match rt.block_on(core.gtp_client.genmove(&c)) {
        Ok((vertex, winrate, lead)) => {
            let v_bytes = vertex.as_bytes();
            let copy_len = v_bytes.len().min(out_vertex_len as usize - 1);
            std::ptr::copy_nonoverlapping(v_bytes.as_ptr(), out_vertex as *mut u8, copy_len);
            *out_vertex.add(copy_len) = 0;

            if !out_winrate.is_null() {
                *out_winrate = winrate.unwrap_or(-1.0);
            }
            if !out_lead.is_null() {
                *out_lead = lead.unwrap_or(f64::NAN);
            }

            let color_enum = Color::from_str(&c).unwrap_or(Color::Black);
            core.game_state.record_move(color_enum, &vertex).ok();
            core.move_history.push(MoveRecord {
                move_number: core.game_state.move_count(),
                color: color_enum,
                vertex,
                captured: vec![],
                winrate,
                lead,
            });

            core.analysis.winrate_black = winrate.unwrap_or(0.5);
            core.analysis.lead = lead.unwrap_or(0.0);
            core.analysis.move_count = core.game_state.move_count();
            core.analysis.current_player = core.game_state.current_player().as_str().to_string();

            0
        }
        Err(e) => {
            core.last_error = e.to_string();
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn go_core_last_error() -> *mut std::os::raw::c_char {
    let msg = match INSTANCE.lock() {
        Ok(inst) => inst
            .as_ref()
            .map(|c| c.last_error.clone())
            .unwrap_or_default(),
        Err(_) => "lock failed".to_string(),
    };
    std::ffi::CString::new(msg).unwrap_or_default().into_raw()
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

    let frame = core.renderer.render(&core.game_state, core.move_history.all_records());

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
    out_lead: *mut f64,
    out_move_count: *mut i32,
    out_current_player: *mut std::os::raw::c_char,
    out_current_player_len: i32,
    out_captures_black: *mut i32,
    out_captures_white: *mut i32,
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
    if !out_lead.is_null() {
        *out_lead = core.analysis.lead;
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
    if !out_current_player.is_null() {
        let cp = core.game_state.current_player().as_str().as_bytes();
        let copy_len = cp.len().min(out_current_player_len as usize - 1);
        std::ptr::copy_nonoverlapping(cp.as_ptr(), out_current_player as *mut u8, copy_len);
        *out_current_player.add(copy_len) = 0;
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

    let _record = match core.move_history.undo() {
        Some(r) => r,
        None => return 0,
    };

    // Rebuild GameState from scratch
    let board_size = core.game_state.board_size();
    core.game_state = GameState::new(board_size);
    for rec in core.move_history.records_up_to_current() {
        core.game_state
            .record_move(rec.color, &rec.vertex)
            .ok();
    }

    // Update analysis from last record if available
    if let Some(last) = core.move_history.last() {
        core.analysis.winrate_black = last.winrate.unwrap_or(0.5);
        core.analysis.lead = last.lead.unwrap_or(0.0);
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

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    match rt.block_on(core.gtp_client.command("clear_board", Some(30.0))) {
        Ok(_) => 0,
        Err(e) => {
            core.last_error = e.to_string();
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn go_core_set_level(level: i32) -> i32 {
    let (max_visits, pda, allow_resign) = match level {
        0 => (3, -2.5f64, false),
        1 => (5, -2.0, false),
        2 => (15, -1.5, true),
        3 => (30, -1.0, true),
        4 => (60, -0.5, true),
        5 => (120, -0.2, true),
        6 => (200, 0.0, true),
        _ => (60, -0.5, true),
    };

    let mut inst = match INSTANCE.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };
    let core = match inst.as_mut() {
        Some(c) => c,
        None => return -1,
    };

    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return -1,
    };

    let _ = rt.block_on(core.gtp_client.set_param("maxVisits", &max_visits.to_string()));
    let _ = rt.block_on(core.gtp_client.set_param(
        "playoutDoublingAdvantage",
        &format!("{pda}"),
    ));
    let _ = rt.block_on(core.gtp_client.set_param(
        "allowResignation",
        &format!("{allow_resign}"),
    ));
    0
}

// ── Helpers ────────────────────────────────────────────────

/// # Safety: ptr must be a valid null-terminated C string
unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_string_lossy()
        .into_owned()
}
