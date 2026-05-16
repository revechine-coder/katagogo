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
                    let row = gs.board_size().checked_sub(row_num).unwrap_or(0);
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
            (3, 3), (3, 9), (3, 15),
            (9, 3), (9, 9), (9, 15),
            (15, 3), (15, 9), (15, 15),
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
