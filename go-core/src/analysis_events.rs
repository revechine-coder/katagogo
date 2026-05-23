use crate::analysis_parser::GtpAnalysisFrame;
use crate::render_frame::{CRenderFrameView, MoveSuggestion, RenderFrame, RenderFrameBuffer};
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub type FfiAnalysisCallback = extern "C" fn(frame: *const CRenderFrameView);

static ANALYSIS_CALLBACK: Lazy<Mutex<Option<FfiAnalysisCallback>>> = Lazy::new(|| Mutex::new(None));

pub fn set_analysis_callback(callback: Option<FfiAnalysisCallback>) -> i32 {
    match ANALYSIS_CALLBACK.lock() {
        Ok(mut stored) => {
            *stored = callback;
            0
        }
        Err(_) => -1,
    }
}

pub fn clear_analysis_callback() -> i32 {
    set_analysis_callback(None)
}

pub fn dispatch_render_frame(frame: RenderFrameBuffer) {
    let callback = ANALYSIS_CALLBACK.lock().ok().and_then(|stored| *stored);
    if let Some(callback) = callback {
        callback(frame.view() as *const CRenderFrameView);
    }
}

pub fn dispatch_analysis_frame(frame: GtpAnalysisFrame, board_size: u8) {
    let mut suggestions = frame
        .candidates
        .into_iter()
        .enumerate()
        .map(|(order, candidate)| MoveSuggestion {
            col: candidate.coordinate.0,
            row: candidate.coordinate.1,
            winrate: candidate.winrate as f64,
            lead: candidate.score_lead as f64,
            visits: candidate.visits,
            order: order as u32,
        })
        .collect::<Vec<_>>();

    suggestions.sort_by_key(|suggestion| std::cmp::Reverse(suggestion.visits));
    for (order, suggestion) in suggestions.iter_mut().enumerate() {
        suggestion.order = order as u32;
    }

    let render_frame = RenderFrame {
        board_size,
        stones: Vec::new(),
        last_move: None,
        move_labels: Vec::new(),
        star_points: standard_star_points(board_size),
        move_count: 0,
        current_player: "b".to_string(),
        captures_black: 0,
        captures_white: 0,
    };

    dispatch_render_frame(RenderFrameBuffer::from_frame(render_frame, suggestions));
}

fn standard_star_points(board_size: u8) -> Vec<(u8, u8)> {
    if board_size == 19 {
        vec![
            (3, 3),
            (3, 9),
            (3, 15),
            (9, 3),
            (9, 9),
            (9, 15),
            (15, 3),
            (15, 9),
            (15, 15),
        ]
    } else {
        Vec::<(u8, u8)>::new()
    }
}
