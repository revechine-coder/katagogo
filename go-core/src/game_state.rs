use crate::error::{GoCoreError, Result};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Black,
    White,
}

impl Color {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "b" | "black" => Ok(Color::Black),
            "w" | "white" => Ok(Color::White),
            _ => Err(GoCoreError::InvalidArgument(format!(
                "invalid color: {s}"
            ))),
        }
    }

    pub fn opponent(&self) -> Self {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Color::Black => "b",
            Color::White => "w",
        }
    }

    pub fn as_display_name(&self) -> &'static str {
        match self {
            Color::Black => "黑棋",
            Color::White => "白棋",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vertex {
    pub col: u8, // 0-18
    pub row: u8, // 0-18
}

impl Vertex {
    pub fn from_coord(s: &str, board_size: u8) -> Result<Self> {
        let s = s.trim().to_uppercase();
        if s == "PASS" || s == "RESIGN" {
            return Err(GoCoreError::InvalidArgument(format!(
                "special moves not valid as Vertex: {s}"
            )));
        }
        let cols = b"ABCDEFGHJKLMNOPQRST";
        let bytes = s.as_bytes();
        if bytes.is_empty() || bytes.len() < 2 {
            return Err(GoCoreError::InvalidArgument(format!(
                "invalid vertex: {s}"
            )));
        }
        let col_char = bytes[0];
        let col = cols.iter().position(|&c| c == col_char).ok_or_else(|| {
            GoCoreError::InvalidArgument(format!("invalid column: {}", col_char as char))
        })? as u8;
        let row_str = &s[1..];
        let row_num: u8 = row_str.parse().map_err(|_| {
            GoCoreError::InvalidArgument(format!("invalid row: {row_str}"))
        })?;
        let row = board_size.checked_sub(row_num).ok_or_else(|| {
            GoCoreError::InvalidArgument(format!("row out of range: {row_num}"))
        })?;
        if col >= board_size || row >= board_size {
            return Err(GoCoreError::InvalidArgument(format!(
                "vertex outside board: {s} (board size {board_size})"
            )));
        }
        Ok(Vertex { col, row })
    }

    pub fn to_string(&self, board_size: u8) -> String {
        let cols = b"ABCDEFGHJKLMNOPQRST";
        let col_char = cols[self.col as usize] as char;
        let row_num = board_size - self.row;
        format!("{col_char}{row_num}")
    }
}

#[derive(Debug, Clone)]
pub struct MoveRecord {
    pub move_number: u32,
    pub color: Color,
    pub vertex: String,
    pub captured: Vec<String>,
    pub winrate: Option<f64>,
    pub lead: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub board_size: u8,
    stones: Vec<Vec<Option<Color>>>,
    current_player: Color,
    move_count: u32,
    captures_black: u32,
    captures_white: u32,
}

impl GameState {
    pub fn new(board_size: u8) -> Self {
        let size = board_size as usize;
        GameState {
            board_size,
            stones: vec![vec![None; size]; size],
            current_player: Color::Black,
            move_count: 0,
            captures_black: 0,
            captures_white: 0,
        }
    }

    pub fn board_size(&self) -> u8 {
        self.board_size
    }

    pub fn current_player(&self) -> Color {
        self.current_player
    }

    pub fn set_current_player(&mut self, color: Color) {
        self.current_player = color;
    }

    pub fn move_count(&self) -> u32 {
        self.move_count
    }

    pub fn stone_at(&self, col: u8, row: u8) -> Option<Color> {
        if col >= self.board_size || row >= self.board_size {
            return None;
        }
        self.stones[row as usize][col as usize]
    }

    pub fn record_move(&mut self, color: Color, vertex: &str) -> Result<MoveRecord> {
        if vertex.to_uppercase() == "PASS" {
            self.move_count += 1;
            self.current_player = color.opponent();
            return Ok(MoveRecord {
                move_number: self.move_count,
                color,
                vertex: "pass".to_string(),
                captured: vec![],
                winrate: None,
                lead: None,
            });
        }

        let v = Vertex::from_coord(vertex, self.board_size)?;

        if self.stones[v.row as usize][v.col as usize].is_some() {
            return Err(GoCoreError::InvalidArgument(format!(
                "intersection already occupied: {}",
                v.to_string(self.board_size)
            )));
        }

        self.stones[v.row as usize][v.col as usize] = Some(color);
        self.move_count += 1;
        self.current_player = color.opponent();

        Ok(MoveRecord {
            move_number: self.move_count,
            color,
            vertex: v.to_string(self.board_size),
            captured: vec![],
            winrate: None,
            lead: None,
        })
    }

    pub fn remove_stone(&mut self, vertex: &str) {
        if let Ok(v) = Vertex::from_coord(vertex, self.board_size) {
            let r = v.row as usize;
            let c = v.col as usize;
            if r < self.board_size as usize && c < self.board_size as usize {
                self.stones[r][c] = None;
            }
        }
    }

    pub fn reset(&mut self) {
        let size = self.board_size as usize;
        self.stones = vec![vec![None; size]; size];
        self.current_player = Color::Black;
        self.move_count = 0;
        self.captures_black = 0;
        self.captures_white = 0;
    }

    pub fn captures(&self) -> (u32, u32) {
        (self.captures_black, self.captures_white)
    }
}
