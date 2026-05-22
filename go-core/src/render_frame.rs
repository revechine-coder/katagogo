use crate::game_state::{Color, GameState, MoveRecord};

#[derive(Debug, Clone)]
pub struct RenderFrame {
    pub board_size: u8,
    pub stones: Vec<StoneRender>,
    pub last_move: Option<(u8, u8)>,
    pub move_labels: Vec<MoveLabel>,
    pub star_points: Vec<(u8, u8)>,
    pub move_count: u32,
    pub current_player: String,
    pub captures_black: u32,
    pub captures_white: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct StarPoint {
    pub col: u8,
    pub row: u8,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct StoneRender {
    pub col: u8,
    pub row: u8,
    pub color: u8, // 0 = black, 1 = white
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct MoveLabel {
    pub col: u8,
    pub row: u8,
    pub move_number: u32,
    pub is_last: u8, // 0 or 1
}

#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct MoveSuggestion {
    pub col: u8,
    pub row: u8,
    pub winrate: f64,
    pub lead: f64,
    pub visits: u32,
    pub order: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CReviewedMove {
    pub move_number: u32,
    pub color: u8,   // 0 = black, 1 = white
    pub quality: u8, // 0 = good, 1 = bad_move, 2 = slack_move
    pub _pad1: [u8; 2],
    pub vertex: [u8; 8], // null-terminated GTP vertex, e.g. "D4\0"
    pub winrate_before: f64,
    pub winrate_after: f64,
    pub score_before: f64,
    pub score_after: f64,
    pub winrate_drop: f64,
    pub score_drop: f64,
    pub ai_best_col: u8,
    pub ai_best_row: u8,
    pub has_ai_best: u8,
    pub suggestions_count: u8,
    pub _pad2: [u8; 4],
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CRenderFrameView {
    pub board_size: u8,
    pub current_player: u8, // 0 = black, 1 = white
    pub _padding: [u8; 2],
    pub move_count: u32,
    pub captures_black: u32,
    pub captures_white: u32,
    pub last_move_col: i16,
    pub last_move_row: i16,
    pub stones: *const StoneRender,
    pub stones_len: usize,
    pub move_labels: *const MoveLabel,
    pub move_labels_len: usize,
    pub star_points: *const StarPoint,
    pub star_points_len: usize,
    pub suggestions: *const MoveSuggestion,
    pub suggestions_len: usize,
}

#[derive(Debug)]
pub struct RenderFrameBuffer {
    frame: CRenderFrameView,
    stones: Vec<StoneRender>,
    move_labels: Vec<MoveLabel>,
    star_points: Vec<StarPoint>,
    suggestions: Vec<MoveSuggestion>,
}

// SAFETY: RenderFrameBuffer owns the Vec allocations that its CRenderFrameView
// points into. Moving the buffer between threads does not move those heap
// allocations. Mutation is mediated by the GoCore mutex; callers that hold a
// raw FFI view must treat it as invalid after the next frame refresh/destroy.
unsafe impl Send for RenderFrameBuffer {}

impl RenderFrameBuffer {
    pub fn from_frame(frame: RenderFrame, suggestions: Vec<MoveSuggestion>) -> Self {
        let stones = frame.stones;
        let move_labels = frame.move_labels;
        let star_points = frame
            .star_points
            .into_iter()
            .map(|(col, row)| StarPoint { col, row })
            .collect::<Vec<_>>();
        let (last_move_col, last_move_row) = frame
            .last_move
            .map(|(col, row)| (col as i16, row as i16))
            .unwrap_or((-1, -1));
        let current_player = match frame.current_player.as_str() {
            "w" | "white" => 1,
            _ => 0,
        };

        let mut buffer = RenderFrameBuffer {
            frame: CRenderFrameView {
                board_size: frame.board_size,
                current_player,
                _padding: [0, 0],
                move_count: frame.move_count,
                captures_black: frame.captures_black,
                captures_white: frame.captures_white,
                last_move_col,
                last_move_row,
                stones: std::ptr::null(),
                stones_len: 0,
                move_labels: std::ptr::null(),
                move_labels_len: 0,
                star_points: std::ptr::null(),
                star_points_len: 0,
                suggestions: std::ptr::null(),
                suggestions_len: 0,
            },
            stones,
            move_labels,
            star_points,
            suggestions,
        };
        buffer.refresh_view_pointers();
        buffer
    }

    pub fn view(&self) -> &CRenderFrameView {
        &self.frame
    }

    fn refresh_view_pointers(&mut self) {
        self.frame.stones = ptr_or_null(&self.stones);
        self.frame.stones_len = self.stones.len();
        self.frame.move_labels = ptr_or_null(&self.move_labels);
        self.frame.move_labels_len = self.move_labels.len();
        self.frame.star_points = ptr_or_null(&self.star_points);
        self.frame.star_points_len = self.star_points.len();
        self.frame.suggestions = ptr_or_null(&self.suggestions);
        self.frame.suggestions_len = self.suggestions.len();
    }
}

fn ptr_or_null<T>(values: &[T]) -> *const T {
    if values.is_empty() {
        std::ptr::null()
    } else {
        values.as_ptr()
    }
}

pub struct BoardRenderer;

impl BoardRenderer {
    pub fn new() -> Self {
        BoardRenderer
    }

    pub fn render(&self, gs: &GameState, move_history: &[MoveRecord]) -> RenderFrame {
        let size = gs.board_size as usize;
        let mut stones = Vec::new();
        let mut move_labels = Vec::new();
        let mut last_move = None;

        for row in 0..size {
            for col in 0..size {
                if let Some(color) = gs.stone_at(col as u8, row as u8) {
                    let c = match color {
                        Color::Black => 0u8,
                        Color::White => 1u8,
                    };
                    stones.push(StoneRender {
                        col: col as u8,
                        row: row as u8,
                        color: c,
                    });
                }
            }
        }

        let cols = b"ABCDEFGHJKLMNOPQRST";
        let total_moves = move_history.len();
        for record in move_history.iter() {
            let vertex_upper = record.vertex.to_uppercase();
            if vertex_upper == "PASS" || vertex_upper == "RESIGN" {
                continue;
            }
            if let Some(col_pos) = cols.iter().position(|&c| c == vertex_upper.as_bytes()[0]) {
                let col = col_pos as u8;
                let row_str = &vertex_upper[1..];
                if let Ok(row_num) = row_str.parse::<u8>() {
                    let row = gs.board_size().saturating_sub(row_num);
                    let is_last = record.move_number as usize == total_moves;
                    move_labels.push(MoveLabel {
                        col,
                        row,
                        move_number: record.move_number,
                        is_last: if is_last { 1 } else { 0 },
                    });
                    if is_last {
                        last_move = Some((col, row));
                    }
                }
            }
        }

        let star_points = vec![
            (3, 3),
            (3, 9),
            (3, 15),
            (9, 3),
            (9, 9),
            (9, 15),
            (15, 3),
            (15, 9),
            (15, 15),
        ];

        RenderFrame {
            board_size: gs.board_size(),
            stones,
            last_move,
            move_labels,
            star_points,
            move_count: gs.move_count(),
            current_player: gs.current_player().as_str().to_string(),
            captures_black: gs.captures().0,
            captures_white: gs.captures().1,
        }
    }
}

impl Default for BoardRenderer {
    fn default() -> Self {
        Self::new()
    }
}
