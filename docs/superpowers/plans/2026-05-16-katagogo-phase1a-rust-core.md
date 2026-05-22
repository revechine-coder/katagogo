# KataGoGo macOS Client — Phase 1A: Rust Shared Core

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust shared core library (`go_core`) that implements game state, move history, GTP client, analysis data, and C ABI exports. Testable via `cargo test` without any GUI.

**Architecture:** Modular Rust crate with `staticlib` + `cdylib` output. Each module in its own file with a thin `ffi.rs` C ABI layer. GTP client spawns real KataGo subprocess for integration tests (or can be mocked for unit tests).

**Tech Stack:** Rust (stable), tokio for process IO, regex for stderr parsing, C ABI for Swift FFI

---

## File Structure

```
~/go-cross-platform/go-core/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Module declarations + C ABI re-exports
│   ├── game_state.rs       # GameState: board array, play(), reset()
│   ├── move_history.rs     # MoveHistory: stack with undo/jump
│   ├── gtp_client.rs       # KataGo subprocess: start/play/genmove/close + stderr parsing
│   ├── analysis.rs         # AnalysisData struct
│   ├── render_frame.rs     # RenderFrame struct + BoardRenderer
│   ├── error.rs            # GoCoreError enum
│   └── ffi.rs              # C ABI extern "C" functions (thin wrappers)
├── tests/
│   ├── game_state_test.rs
│   ├── move_history_test.rs
│   └── gtp_client_test.rs  # integration test (needs KataGo binary)
|   └── render_frame_test.rs
└── Makefile                # build + test + copy-to-xcode
```

### Task 1: Project scaffold + Cargo.toml

**Files:**
- Create: `~/go-cross-platform/go-core/Cargo.toml`
- Create: `~/go-cross-platform/go-core/src/lib.rs`

- [ ] **Step 1: Write Cargo.toml**

```toml
[package]
name = "go-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib"]
name = "go_core"

[dependencies]
tokio = { version = "1", features = ["process", "io-util", "sync"] }
regex = "1"
thiserror = "2"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Write src/lib.rs with module declarations**

```rust
pub mod error;
pub mod game_state;
pub mod move_history;
pub mod gtp_client;
pub mod analysis;
pub mod render_frame;
pub mod ffi;
```

- [ ] **Step 3: Create Makefile**

```makefile
.PHONY: all build test clean

all: build test

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

# Copy static lib to Xcode project
install: build
	@mkdir -p ../KataGoGo/SharedCore
	cp target/release/libgo_core.a ../KataGoGo/SharedCore/
	cp src/ffi.h ../KataGoGo/SharedCore/ 2>/dev/null || true
	@echo "Installed libgo_core.a to Xcode project"
```

- [ ] **Step 4: Verify scaffold**

Run: `cd ~/go-cross-platform/go-core && cargo build 2>&1`
Expected: Build succeeds (warnings about dead code are OK at this stage)

- [ ] **Step 5: Commit**

```bash
cd ~/go-cross-platform/go-core
git init && git add -A && git commit -m "feat: scaffold go-core Rust crate"
```

---

### Task 2: Error type

**Files:**
- Create: `~/go-cross-platform/go-core/src/error.rs`

- [ ] **Step 1: Write error.rs**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GoCoreError {
    #[error("KataGo process not running")]
    ProcessNotRunning,

    #[error("KataGo process pipe closed")]
    PipeClosed,

    #[error("KataGo command timed out: {0}")]
    Timeout(String),

    #[error("KataGo rejected command: {0}")]
    Rejected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, GoCoreError>;
```

- [ ] **Step 2: Verify it compiles**

Run: `cd ~/go-cross-platform/go-core && cargo build 2>&1`
Expected: Success

- [ ] **Step 3: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: add error types"
```

---

### Task 3: GameState module

**Files:**
- Create: `~/go-cross-platform/go-core/src/game_state.rs`

- [ ] **Step 1: Write game_state.rs**

```rust
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
    pub col: u8, // 0-18 (A-T skipping I)
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
        GameState {
            board_size,
            stones: vec![vec![None; board_size as usize]; board_size as usize],
            current_player: Color::Black,
            move_count: 0,
            captures_black: 0,
            captures_white: 0,
        }
    }

    pub fn current_player(&self) -> Color {
        self.current_player
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

    /// Record a move. Does NOT validate board rules — that's KataGo's job.
    /// Returns MoveRecord with empty captured/analysis fields.
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

    /// Remove captured stones (called with data from KataGo's response).
    pub fn remove_captures(&mut self, color: Color, captured: &[String]) {
        let board_size = self.board_size as usize;
        for cap in captured {
            if let Ok(v) = Vertex::from_coord(cap, self.board_size) {
                let r = v.row as usize;
                let c = v.col as usize;
                if r < board_size && c < board_size {
                    if self.stones[r][c].is_some() {
                        self.stones[r][c] = None;
                        match color {
                            Color::Black => self.captures_white += 1,
                            Color::White => self.captures_black += 1,
                        }
                    }
                }
            }
        }
    }

    /// Remove a single stone (for undo).
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
```

- [ ] **Step 2: Write test**

```rust
// tests/game_state_test.rs
use go_core::game_state::*;

#[test]
fn test_basic_play() {
    let mut gs = GameState::new(19);
    assert_eq!(gs.move_count(), 0);
    assert_eq!(gs.current_player(), Color::Black);

    let rec = gs.record_move(Color::Black, "D4").unwrap();
    assert_eq!(rec.vertex, "D4");
    assert_eq!(rec.move_number, 1);
    assert_eq!(gs.stone_at(3, 15), Some(Color::Black));
    assert_eq!(gs.current_player(), Color::White);
}

#[test]
fn test_occupied_intersection() {
    let mut gs = GameState::new(19);
    gs.record_move(Color::Black, "D4").unwrap();
    let result = gs.record_move(Color::White, "D4");
    assert!(result.is_err());
}

#[test]
fn test_pass() {
    let mut gs = GameState::new(19);
    let rec = gs.record_move(Color::Black, "pass").unwrap();
    assert_eq!(rec.vertex, "pass");
    assert_eq!(gs.current_player(), Color::White);
}

#[test]
fn test_reset() {
    let mut gs = GameState::new(19);
    gs.record_move(Color::Black, "D4").unwrap();
    gs.record_move(Color::White, "Q16").unwrap();
    assert_eq!(gs.move_count(), 2);
    gs.reset();
    assert_eq!(gs.move_count(), 0);
    assert_eq!(gs.stone_at(3, 15), None);
}

#[test]
fn test_color_from_str() {
    assert_eq!(Color::from_str("b").unwrap(), Color::Black);
    assert_eq!(Color::from_str("black").unwrap(), Color::Black);
    assert_eq!(Color::from_str("w").unwrap(), Color::White);
    assert_eq!(Color::from_str("white").unwrap(), Color::White);
    assert!(Color::from_str("x").is_err());
}

#[test]
fn test_vertex_coord() {
    let v = Vertex::from_coord("D4", 19).unwrap();
    assert_eq!(v.col, 3);
    assert_eq!(v.row, 15);
    assert_eq!(v.to_string(19), "D4");
}

#[test]
fn test_vertex_out_of_bounds() {
    assert!(Vertex::from_coord("Z1", 19).is_err());
    assert!(Vertex::from_coord("A20", 19).is_err());
}

#[test]
fn test_small_board() {
    let mut gs = GameState::new(9);
    gs.record_move(Color::Black, "E5").unwrap();
    assert_eq!(gs.stone_at(4, 4), Some(Color::Black));
}
```

- [ ] **Step 3: Run tests**

Run: `cd ~/go-cross-platform/go-core && cargo test game_state -- --nocapture 2>&1`
Expected: All 8 tests PASS

- [ ] **Step 4: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: GameState with play/pass/reset"
```

---

### Task 4: MoveHistory module

**Files:**
- Create: `~/go-cross-platform/go-core/src/move_history.rs`

- [ ] **Step 1: Write move_history.rs**

```rust
use crate::game_state::{Color, MoveRecord};
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct MoveHistory {
    records: Vec<MoveRecord>,
    current_index: usize,  // points to next record to show, 0 = empty
}

impl MoveHistory {
    pub fn new() -> Self {
        MoveHistory {
            records: Vec::new(),
            current_index: 0,
        }
    }

    pub fn push(&mut self, record: MoveRecord) {
        // If we've undone some moves, truncate future history
        if self.current_index < self.records.len() {
            self.records.truncate(self.current_index);
        }
        self.records.push(record);
        self.current_index = self.records.len();
    }

    /// Undo the last move. Returns the undone record, or None if already at start.
    pub fn undo(&mut self) -> Option<MoveRecord> {
        if self.current_index == 0 {
            return None;
        }
        self.current_index -= 1;
        Some(self.records[self.current_index].clone())
    }

    /// Redo a previously undone move. Returns the redone record, or None.
    pub fn redo(&mut self) -> Option<MoveRecord> {
        if self.current_index >= self.records.len() {
            return None;
        }
        let record = self.records[self.current_index].clone();
        self.current_index += 1;
        Some(record)
    }

    /// Jump to a specific move number (1-based). Returns the records that need to be undone
    /// (if jumping backwards) or applies forward jumps by returning empty vec.
    pub fn jump_to(&mut self, move_number: u32) -> Vec<MoveRecord> {
        if move_number == 0 {
            let undone: Vec<MoveRecord> = self.records[..self.current_index].to_vec();
            self.current_index = 0;
            return undone;
        }
        if move_number as usize > self.records.len() {
            self.current_index = self.records.len();
            return vec![];
        }
        if (move_number as usize) < self.current_index {
            // Going backward
            let undone: Vec<MoveRecord> =
                self.records[move_number as usize..self.current_index].to_vec();
            self.current_index = move_number as usize;
            undone
        } else {
            // Going forward
            self.current_index = move_number as usize;
            vec![]
        }
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn last(&self) -> Option<&MoveRecord> {
        if self.current_index == 0 {
            return None;
        }
        self.records.get(self.current_index - 1)
    }

    pub fn all_records(&self) -> &[MoveRecord] {
        &self.records
    }

    pub fn records_up_to_current(&self) -> &[MoveRecord] {
        &self.records[..self.current_index]
    }

    pub fn reset(&mut self) {
        self.records.clear();
        self.current_index = 0;
    }
}
```

- [ ] **Step 2: Write test**

```rust
// tests/move_history_test.rs
use go_core::game_state::{Color, MoveRecord};
use go_core::move_history::MoveHistory;

fn dummy_record(move_number: u32) -> MoveRecord {
    MoveRecord {
        move_number,
        color: if move_number % 2 == 1 { Color::Black } else { Color::White },
        vertex: if move_number % 2 == 1 { "D4" } else { "Q16" }.to_string(),
        captured: vec![],
        winrate: None,
        lead: None,
    }
}

#[test]
fn test_push_and_undo() {
    let mut hist = MoveHistory::new();
    assert!(hist.undo().is_none());

    hist.push(dummy_record(1));
    hist.push(dummy_record(2));

    let undone = hist.undo().unwrap();
    assert_eq!(undone.move_number, 2);
    assert_eq!(hist.current_index(), 1);

    let redo = hist.redo().unwrap();
    assert_eq!(redo.move_number, 2);
    assert_eq!(hist.current_index(), 2);
}

#[test]
fn test_jump_to_start() {
    let mut hist = MoveHistory::new();
    for i in 1..=5 {
        hist.push(dummy_record(i));
    }
    let undone = hist.jump_to(0);
    assert_eq!(undone.len(), 5);
    assert_eq!(hist.current_index(), 0);
    assert!(hist.undo().is_none());
}

#[test]
fn test_jump_to_middle() {
    let mut hist = MoveHistory::new();
    for i in 1..=5 {
        hist.push(dummy_record(i));
    }
    let undone = hist.jump_to(3);
    assert_eq!(undone.len(), 2); // records 4,5 undone
    assert_eq!(hist.current_index(), 3);
    assert_eq!(hist.last().unwrap().move_number, 3);
}

#[test]
fn test_truncate_on_push_after_undo() {
    let mut hist = MoveHistory::new();
    for i in 1..=3 {
        hist.push(dummy_record(i));
    }
    hist.undo();
    hist.push(dummy_record(4));
    assert_eq!(hist.len(), 3);
    assert_eq!(hist.last().unwrap().move_number, 4);
}

#[test]
fn test_empty_history() {
    let hist = MoveHistory::new();
    assert!(hist.last().is_none());
    assert_eq!(hist.len(), 0);
}
```

- [ ] **Step 3: Run tests**

Run: `cd ~/go-cross-platform/go-core && cargo test move_history 2>&1`
Expected: All 5 tests PASS

- [ ] **Step 4: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: MoveHistory with undo/redo/jump"
```

---

### Task 5: GTP Client

**Files:**
- Create: `~/go-cross-platform/go-core/src/gtp_client.rs`

- [ ] **Step 1: Write gtp_client.rs**

```rust
use crate::error::{GoCoreError, Result};
use regex::Regex;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const ERR_BUF_CAPACITY: usize = 500;

pub struct GtpClient {
    process: Option<Child>,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
}

impl GtpClient {
    pub fn new() -> Self {
        GtpClient {
            process: None,
            stderr_lines: Arc::new(Mutex::new(VecDeque::with_capacity(ERR_BUF_CAPACITY))),
        }
    }

    pub async fn start(
        &mut self,
        binary_path: &str,
        config_path: &str,
        model_path: &str,
        board_size: u8,
        timeout_secs: f64,
    ) -> Result<()> {
        let mut child = Command::new(binary_path)
            .arg("gtp")
            .arg("-config")
            .arg(config_path)
            .arg("-model")
            .arg(model_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| GoCoreError::Io(e))?;

        let stderr = child.stderr.take().ok_or(GoCoreError::PipeClosed)?;
        let stderr_lines = self.stderr_lines.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut buf = stderr_lines.lock().await;
                if buf.len() >= ERR_BUF_CAPACITY {
                    buf.pop_front();
                }
                buf.push_back(line);
            }
        });

        self.process = Some(child);
        self.command(&format!("boardsize {board_size}"), Some(timeout_secs))
            .await?;
        self.command("clear_board", Some(timeout_secs)).await?;
        Ok(())
    }

    pub async fn play(&mut self, color: &str, vertex: &str) -> Result<()> {
        self.command(&format!("play {color} {vertex}"), Some(60.0))
            .await?;
        Ok(())
    }

    pub async fn genmove(
        &mut self,
        color: &str,
    ) -> Result<(String, Option<f64>, Option<f64>)> {
        let response = self.command(&format!("genmove {color}"), Some(120.0)).await?;
        let vertex = response.trim().to_lowercase();

        let mut stderr_lines = self.stderr_lines.lock().await;
        let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
        drop(stderr_lines);

        let winrate = Self::parse_winrate(&all_stderr, &vertex, color);
        let lead = Self::parse_lead(&all_stderr, color);

        // Retry with sleep if winrate not immediately available
        let winrate = if winrate.is_none() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let stderr_lines = self.stderr_lines.lock().await;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            Self::parse_winrate(&all_stderr, &vertex, color)
        } else {
            winrate
        };

        let lead = if lead.is_none() {
            let stderr_lines = self.stderr_lines.lock().await;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            Self::parse_lead(&all_stderr, color)
        } else {
            lead
        };

        Ok((vertex, winrate, lead))
    }

    pub async fn final_score(&mut self) -> Result<String> {
        let response = self.command("final_score", Some(120.0)).await?;
        Ok(response.trim().to_string())
    }

    pub async fn set_param(&mut self, key: &str, value: &str) -> Result<()> {
        self.command(
            &format!("kata-set-param {key} {value}"),
            Some(10.0),
        )
        .await?;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(ref mut process) = self.process {
            if let Some(stdin) = process.stdin.as_mut() {
                let _ = stdin.write_all(b"quit\n").await;
                let _ = stdin.flush().await;
            }
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(3),
                process.wait(),
            )
            .await;
        }
        self.process = None;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.process
            .as_ref()
            .map(|p| p.try_status().ok().is_none())
            .unwrap_or(false)
    }

    // ── Low-level command ──────────────────────────────────

    async fn command(&mut self, cmd: &str, timeout: Option<f64>) -> Result<String> {
        let process = self
            .process
            .as_mut()
            .ok_or(GoCoreError::ProcessNotRunning)?;

        let stdin = process
            .stdin
            .as_mut()
            .ok_or(GoCoreError::PipeClosed)?;
        let stdout = process
            .stdout
            .as_mut()
            .ok_or(GoCoreError::PipeClosed)?;

        // Write command
        let cmd_with_nl = format!("{cmd}\n");
        stdin
            .write_all(cmd_with_nl.as_bytes())
            .await
            .map_err(|_| GoCoreError::PipeClosed)?;
        stdin.flush().await.map_err(|_| GoCoreError::PipeClosed)?;

        // Read response
        let fut = async {
            let mut reader = BufReader::new(stdout);
            let mut response = String::new();
            loop {
                let mut line = String::new();
                reader
                    .read_line(&mut line)
                    .await
                    .map_err(|_| GoCoreError::PipeClosed)?;
                if line.is_empty() {
                    return Err(GoCoreError::PipeClosed);
                }
                let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                if trimmed.starts_with('=') {
                    response.push_str(trimmed[1..].trim());
                    return Ok(response.trim().to_string());
                } else if trimmed.starts_with('?') {
                    let msg = trimmed[1..].trim();
                    return Err(GoCoreError::Rejected(msg.to_string()));
                } else {
                    response.push_str(trimmed);
                    response.push('\n');
                }
            }
        };

        let result = if let Some(t) = timeout {
            tokio::time::timeout(std::time::Duration::from_secs_f64(t), fut)
                .await
                .map_err(|_| GoCoreError::Timeout(cmd.to_string()))?
        } else {
            fut.await
        };

        result
    }

    // ── Stderr parsing ─────────────────────────────────────

    fn parse_winrate(lines: &[String], vertex: &str, color: &str) -> Option<f64> {
        let pattern = Regex::new(&format!(
            r#"(?i)\b{re_vertex}\b.*"#,
            re_vertex = regex::escape(vertex)
        ))
        .ok()?;
        let winrate_re = Regex::new(r"\bP\s+(\d+\.\d+)%").ok()?;

        for line in lines.iter().rev() {
            if pattern.is_match(line) {
                if let Some(caps) = winrate_re.captures(line) {
                    let wr: f64 = caps[1].parse().ok()?;
                    let wr = wr / 100.0;
                    return if color.eq_ignore_ascii_case("w")
                        || color.eq_ignore_ascii_case("white")
                    {
                        Some(1.0 - wr)
                    } else {
                        Some(wr)
                    };
                }
            }
        }
        None
    }

    fn parse_lead(lines: &[String], color: &str) -> Option<f64> {
        let lead_re = Regex::new(r"\bS\s+([-\d.]+)c\b").ok()?;
        for line in lines.iter().rev() {
            if let Some(caps) = lead_re.captures(line) {
                let val: f64 = caps[1].parse().ok()?;
                let val = val / 100.0; // centipoints to points
                return if color.eq_ignore_ascii_case("w") || color.eq_ignore_ascii_case("white")
                {
                    Some(-val)
                } else {
                    Some(val)
                };
            }
        }
        None
    }
}
```

- [ ] **Step 2: Write integration test**

```rust
// tests/gtp_client_test.rs
use go_core::gtp_client::GtpClient;

const KATAGO_BIN: &str = "/opt/kata-go-app/engine/katago";
const KATAGO_CFG: &str = "/opt/kata-go-app/engine/gtp.cfg";
const KATAGO_MODEL: &str =
    "/opt/kata-go-app/engine/models/lionffen_b24c64.bin.gz";

#[tokio::test]
async fn test_start_and_play() {
    let mut client = GtpClient::new();
    client
        .start(KATAGO_BIN, KATAGO_CFG, KATAGO_MODEL, 19, 120.0)
        .await
        .expect("Failed to start KataGo");

    assert!(client.is_running());

    client.play("b", "D4").await.expect("play failed");
    let (vertex, winrate, lead) = client
        .genmove("w")
        .await
        .expect("genmove failed");
    assert!(!vertex.is_empty(), "genmove returned empty vertex");
    println!("AI played: {vertex}, winrate={winrate:?}, lead={lead:?}");

    let score = client.final_score().await.expect("final_score failed");
    println!("Score: {score}");

    client.close().await.expect("close failed");
}
```

- [ ] **Step 3: Check if KataGo paths exist**

Run: `ls -la /opt/kata-go-app/engine/katago /opt/kata-go-app/engine/gtp.cfg /opt/kata-go-app/engine/models/lionffen_b24c64.bin.gz 2>&1`

Expected: All three exist

- [ ] **Step 4: Run build (skip integration test for now)**

Run: `cd ~/go-cross-platform/go-core && cargo build 2>&1`
Expected: Build succeeds

- [ ] **Step 5: Run integration test**

Run: `cd ~/go-cross-platform/go-core && cargo test gtp_client -- --nocapture --test-threads=1 2>&1`
Expected: Test passes, AI plays a move, prints vertex/winrate/lead

- [ ] **Step 6: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: GtpClient with stderr winrate/lead parsing"
```

---

### Task 6: RenderFrame + BoardRenderer

**Files:**
- Create: `~/go-cross-platform/go-core/src/render_frame.rs`

- [ ] **Step 1: Write render_frame.rs**

```rust
use crate::game_state::{Color, GameState};

/// Serializable rendering data — SwiftUI just paints this.
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
pub struct StoneRender {
    pub col: u8,
    pub row: u8,
    pub color: u8, // 0 = black, 1 = white
}

#[derive(Debug, Clone)]
pub struct MoveLabel {
    pub col: u8,
    pub row: u8,
    pub move_number: u32,
    pub is_last: bool,
}

pub struct BoardRenderer;

impl BoardRenderer {
    pub fn new() -> Self {
        BoardRenderer
    }

    /// Generate a RenderFrame from the current GameState.
    pub fn render(
        &self,
        gs: &GameState,
        move_history: &[crate::move_history::MoveRecord],
    ) -> RenderFrame {
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

        // Move labels from move history
        for record in move_history.iter() {
            let vertex_upper = record.vertex.to_uppercase();
            if vertex_upper == "PASS" || vertex_upper == "RESIGN" {
                continue;
            }
            let cols = b"ABCDEFGHJKLMNOPQRST";
            if let Some(col_pos) = cols.iter().position(|&c| c == vertex_upper.as_bytes()[0]) {
                let col = col_pos as u8;
                let row_str = &vertex_upper[1..];
                if let Ok(row_num) = row_str.parse::<u8>() {
                    let row = gs.board_size.checked_sub(row_num).unwrap_or(0);
                    let is_last = record.move_number == move_history.len() as u32
                        || (move_history.last().map(|r| r.move_number) == Some(record.move_number));
                    move_labels.push(MoveLabel {
                        col,
                        row,
                        move_number: record.move_number,
                        is_last,
                    });
                    if is_last {
                        last_move = Some((col, row));
                    }
                }
            }
        }

        // Standard star points for 19x19
        let star_points = vec![
            (3, 3), (3, 9), (3, 15),
            (9, 3), (9, 9), (9, 15),
            (15, 3), (15, 9), (15, 15),
        ];

        RenderFrame {
            board_size: gs.board_size,
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
```

- [ ] **Step 2: Write test**

```rust
// tests/render_frame_test.rs
use go_core::game_state::*;
use go_core::move_history::MoveHistory;
use go_core::render_frame::*;

#[test]
fn test_empty_board_render() {
    let gs = GameState::new(19);
    let mh = MoveHistory::new();
    let renderer = BoardRenderer::new();
    let frame = renderer.render(&gs, mh.all_records());
    assert_eq!(frame.board_size, 19);
    assert!(frame.stones.is_empty());
    assert!(frame.last_move.is_none());
    assert_eq!(frame.star_points.len(), 9);
}

#[test]
fn test_with_moves() {
    let mut gs = GameState::new(19);
    let mut mh = MoveHistory::new();
    gs.record_move(Color::Black, "D4").unwrap();
    mh.push(MoveRecord {
        move_number: 1,
        color: Color::Black,
        vertex: "D4".to_string(),
        captured: vec![],
        winrate: None,
        lead: None,
    });
    let renderer = BoardRenderer::new();
    let frame = renderer.render(&gs, mh.all_records());
    assert_eq!(frame.stones.len(), 1);
    assert_eq!(frame.move_labels.len(), 1);
    assert_eq!(frame.last_move, Some((3, 15)));
    assert_eq!(frame.move_labels[0].move_number, 1);
}
```

- [ ] **Step 3: Run tests**

Run: `cd ~/go-cross-platform/go-core && cargo test render_frame 2>&1`
Expected: Both tests PASS

- [ ] **Step 4: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: RenderFrame + BoardRenderer"
```

---

### Task 7: FFI (C ABI exports)

**Files:**
- Create: `~/go-cross-platform/go-core/src/ffi.rs`

- [ ] **Step 1: Write ffi.rs**

```rust
use crate::analysis::AnalysisData;
use crate::error::GoCoreError;
use crate::game_state::{Color, GameState, MoveRecord};
use crate::gtp_client::GtpClient;
use crate::move_history::MoveHistory;
use crate::render_frame::{BoardRenderer, RenderFrame, StoneRender, MoveLabel};
use std::ffi::{CStr, CString};
use std::sync::Mutex;
use std::os::raw::c_char;

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

static INSTANCE: once_cell::sync::Lazy<Mutex<Option<GoCore>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

// ── FFI Helpers ────────────────────────────────────────────

fn set_error(msg: String) {
    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            core.last_error = msg;
        }
    }
}

/// # Safety: ptr must be a valid null-terminated C string
unsafe fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

fn string_to_cstring(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

// ── FFI Exports ────────────────────────────────────────────

/// Create the GoCore engine. Must be called once before other functions.
#[no_mangle]
pub extern "C" fn go_core_create() -> i32 {
    if let Ok(mut inst) = INSTANCE.lock() {
        *inst = Some(GoCore::new());
        0
    } else {
        -1
    }
}

/// Destroy the engine and free resources.
#[no_mangle]
pub extern "C" fn go_core_destroy() -> i32 {
    if let Ok(mut inst) = INSTANCE.lock() {
        // Drop the GtpClient which closes the subprocess
        *inst = None;
        0
    } else {
        -1
    }
}

/// Start the KataGo subprocess.
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn go_core_start(
    binary_path: *const c_char,
    config_path: *const c_char,
    model_path: *const c_char,
    board_size: i32,
    timeout_secs: f64,
) -> i32 {
    let bin = cstr_to_string(binary_path);
    let cfg = cstr_to_string(config_path);
    let mdl = cstr_to_string(model_path);

    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(core.gtp_client.start(
                &bin, &cfg, &mdl, board_size as u8, timeout_secs,
            )) {
                Ok(_) => {
                    core.game_state = GameState::new(board_size as u8);
                    core.move_history = MoveHistory::new();
                    0
                }
                Err(e) => {
                    set_error(e.to_string());
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Close the KataGo subprocess.
#[no_mangle]
pub extern "C" fn go_core_close() -> i32 {
    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(core.gtp_client.close()) {
                Ok(_) => 0,
                Err(e) => {
                    set_error(e.to_string());
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Play a move on the board AND send to KataGo.
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn go_core_play(
    color: *const c_char,
    vertex: *const c_char,
) -> i32 {
    let c = cstr_to_string(color);
    let v = cstr_to_string(vertex);

    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            // 1. Send play to KataGo
            let rt = tokio::runtime::Runtime::new().unwrap();
            if let Err(e) = rt.block_on(core.gtp_client.play(&c, &v)) {
                set_error(e.to_string());
                return -1;
            }

            // 2. Record in GameState
            let color_enum = match Color::from_str(&c) {
                Ok(c) => c,
                Err(e) => {
                    set_error(e.to_string());
                    return -1;
                }
            };
            match core.game_state.record_move(color_enum, &v) {
                Ok(record) => {
                    core.move_history.push(record);
                    0
                }
                Err(e) => {
                    set_error(e.to_string());
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Ask KataGo to generate a move. Results stored in analysis.
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn go_core_genmove(
    color: *const c_char,
    out_vertex: *mut c_char,
    out_vertex_len: i32,
    out_winrate: *mut f64,
    out_lead: *mut f64,
) -> i32 {
    let c = cstr_to_string(color);

    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(core.gtp_client.genmove(&c)) {
                Ok((vertex, winrate, lead)) => {
                    // Write to output pointers
                    let v_bytes = vertex.as_bytes();
                    let copy_len = (v_bytes.len()).min(out_vertex_len as usize - 1);
                    std::ptr::copy_nonoverlapping(
                        v_bytes.as_ptr(),
                        out_vertex as *mut u8,
                        copy_len,
                    );
                    *out_vertex.add(copy_len) = 0; // null terminate

                    if !out_winrate.is_null() {
                        *out_winrate = winrate.unwrap_or(-1.0);
                    }
                    if !out_lead.is_null() {
                        *out_lead = lead.unwrap_or(f64::NAN);
                    }

                    // Record in GameState + MoveHistory
                    let color_enum = Color::from_str(&c).unwrap();
                    core.game_state.record_move(color_enum, &vertex).ok();
                    core.move_history.push(MoveRecord {
                        move_number: core.game_state.move_count(),
                        color: color_enum,
                        vertex,
                        captured: vec![],
                        winrate,
                        lead,
                    });

                    // Update analysis
                    core.analysis.winrate_black = winrate.unwrap_or(0.5);
                    core.analysis.lead = lead.unwrap_or(0.0);
                    core.analysis.move_count = core.game_state.move_count();
                    core.analysis.current_player = core.game_state.current_player().as_str().to_string();

                    0
                }
                Err(e) => {
                    set_error(e.to_string());
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Get the last error message.
#[no_mangle]
pub extern "C" fn go_core_last_error() -> *mut c_char {
    if let Ok(inst) = INSTANCE.lock() {
        if let Some(ref core) = *inst {
            return string_to_cstring(&core.last_error);
        }
    }
    string_to_cstring("engine not initialized")
}

/// Get the current render frame. Fills the provided struct.
/// # Safety
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
    out_current_player: *mut c_char,
    out_current_player_len: i32,
) -> i32 {
    if let Ok(inst) = INSTANCE.lock() {
        if let Some(ref core) = *inst {
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

            // Write current player
            if !out_current_player.is_null() {
                let cp = frame.current_player.as_bytes();
                let copy_len = (cp.len()).min(out_current_player_len as usize - 1);
                std::ptr::copy_nonoverlapping(cp.as_ptr(), out_current_player as *mut u8, copy_len);
                *out_current_player.add(copy_len) = 0;
            }

            // Write stones
            let num_stones = frame.stones.len().min(out_max_stones as usize);
            if !out_num_stones.is_null() {
                *out_num_stones = num_stones as i32;
            }
            for i in 0..num_stones {
                *out_stones.add(i) = frame.stones[i].clone();
            }

            // Write move labels
            let num_labels = frame.move_labels.len().min(out_max_labels as usize);
            if !out_num_labels.is_null() {
                *out_num_labels = num_labels as i32;
            }
            for i in 0..num_labels {
                *out_move_labels.add(i) = frame.move_labels[i].clone();
            }

            0
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Get analysis data.
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn go_core_get_analysis(
    out_winrate_black: *mut f64,
    out_lead: *mut f64,
    out_move_count: *mut i32,
    out_current_player: *mut c_char,
    out_current_player_len: i32,
    out_captures_black: *mut i32,
    out_captures_white: *mut i32,
) -> i32 {
    if let Ok(inst) = INSTANCE.lock() {
        if let Some(ref core) = *inst {
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
                let copy_len = (cp.len()).min(out_current_player_len as usize - 1);
                std::ptr::copy_nonoverlapping(cp.as_ptr(), out_current_player as *mut u8, copy_len);
                *out_current_player.add(copy_len) = 0;
            }
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Undo last move pair (player + AI). Returns number of moves undone.
#[no_mangle]
pub extern "C" fn go_core_undo() -> i32 {
    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            let mut undone = 0;
            while let Some(record) = core.move_history.undo() {
                if record.vertex.to_uppercase() != "PASS" {
                    core.game_state.remove_stone(&record.vertex);
                }
                undone += 1;
            }
            // After undo, current_player should be set correctly from GameState
            // GameState doesn't auto-track, so we rebuild from MoveHistory
            core.game_state.reset();
            for rec in core.move_history.records_up_to_current() {
                let color_enum = rec.color;
                core.game_state.record_move(color_enum, &rec.vertex).ok();
            }
            core.game_state = GameState::new(core.game_state.board_size);
            for rec in core.move_history.records_up_to_current() {
                let color_enum = rec.color;
                core.game_state.record_move(color_enum, &rec.vertex).ok();
            }
            undone
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Reset the board.
#[no_mangle]
pub extern "C" fn go_core_reset() -> i32 {
    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            core.game_state.reset();
            core.move_history.reset();
            core.analysis = AnalysisData::new();
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(core.gtp_client.command("clear_board", Some(30.0))) {
                Ok(_) => 0,
                Err(e) => {
                    set_error(e.to_string());
                    -1
                }
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// Set KataGo parameter (for difficulty adjustment).
#[no_mangle]
pub extern "C" fn go_core_set_level(level: i32) -> i32 {
    // level: 0=9k, 1=7k-9k, 2=4k-6k, 3=1k-3k, 4=1d-3d, 5=4d-6d, 6=7d+
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

    if let Ok(mut inst) = INSTANCE.lock() {
        if let Some(ref mut core) = *inst {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let _ = rt.block_on(core.gtp_client.set_param("maxVisits", &max_visits.to_string()));
            let _ = rt.block_on(core.gtp_client.set_param("playoutDoublingAdvantage", &format!("{pda}")));
            let _ = rt.block_on(core.gtp_client.set_param("allowResignation", &format!("{allow_resign}")));
            0
        } else {
            -1
        }
    } else {
        -1
    }
}
```

- [ ] **Step 2: Update lib.rs**

Add `once_cell` dependency to Cargo.toml:
```toml
once_cell = "1"
```

Add `analysis` module to lib.rs (create empty analysis.rs first):

```rust
// src/analysis.rs
pub struct AnalysisData {
    pub winrate_black: f64,
    pub lead: f64,
    pub move_count: u32,
    pub current_player: String,
}

impl AnalysisData {
    pub fn new() -> Self {
        AnalysisData {
            winrate_black: 0.5,
            lead: 0.0,
            move_count: 0,
            current_player: "b".to_string(),
        }
    }
}
```

- [ ] **Step 3: Build and verify**

Run: `cd ~/go-cross-platform/go-core && cargo build --release 2>&1`
Expected: Build succeeds. `target/release/libgo_core.a` exists.

Run: `ls -lh ~/go-cross-platform/go-core/target/release/libgo_core.a`
Expected: Static library file exists (a few MB)

- [ ] **Step 4: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: C ABI FFI exports + analysis"
```

---

### Task 8: C header file for Swift

**Files:**
- Create: `~/go-cross-platform/go-core/src/go_core.h`

- [ ] **Step 1: Write go_core.h**

```c
#ifndef GO_CORE_H
#define GO_CORE_H

#include <stdint.h>

// ── Data structures ────────────────────────────────────────

typedef struct {
    uint8_t col;
    uint8_t row;
    uint8_t color; // 0=black, 1=white
} StoneRender;

typedef struct {
    uint8_t col;
    uint8_t row;
    uint32_t move_number;
    uint8_t is_last; // 0 or 1
} MoveLabel;

// ── Lifecycle ──────────────────────────────────────────────

int go_core_create(void);
int go_core_destroy(void);
int go_core_start(const char* binary_path, const char* config_path,
                  const char* model_path, int board_size, double timeout_secs);
int go_core_close(void);

// ── Gameplay ───────────────────────────────────────────────

int go_core_play(const char* color, const char* vertex);
int go_core_genmove(const char* color,
                    char* out_vertex, int out_vertex_len,
                    double* out_winrate, double* out_lead);
int go_core_undo(void);
int go_core_reset(void);

// ── Settings ───────────────────────────────────────────────

int go_core_set_level(int level); // 0-6

// ── Rendering ──────────────────────────────────────────────

int go_core_get_render_frame(
    StoneRender* out_stones, int out_max_stones, int* out_num_stones,
    MoveLabel* out_move_labels, int out_max_labels, int* out_num_labels,
    int* out_board_size,
    int* out_last_move_col, int* out_last_move_row,
    int* out_move_count,
    char* out_current_player, int out_current_player_len);

// ── Analysis ───────────────────────────────────────────────

int go_core_get_analysis(
    double* out_winrate_black, double* out_lead,
    int* out_move_count,
    char* out_current_player, int out_current_player_len,
    int* out_captures_black, int* out_captures_white);

// ── Error ──────────────────────────────────────────────────

const char* go_core_last_error(void);

#endif // GO_CORE_H
```

- [ ] **Step 2: Verify header compiles with C**

Run: `cd ~/go-cross-platform/go-core && echo '#include "src/go_core.h"' | cc -fsyntax-only -xc - 2>&1`
Expected: No errors

- [ ] **Step 3: Commit**

```bash
cd ~/go-cross-platform/go-core && git add -A && git commit -m "feat: C header for Swift FFI"
```

---

## Verify All Tasks Run

- [ ] Run: `cd ~/go-cross-platform/go-core && cargo test 2>&1`
Expected: All tests pass
- [ ] Run: `cd ~/go-cross-platform/go-core && cargo build --release 2>&1`
Expected: Build succeeds
- [ ] Run: `ls -lh target/release/libgo_core.a`
Expected: File exists

## Output

The Phase 1A deliverable is:

```
~/go-cross-platform/go-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── game_state.rs
│   ├── move_history.rs
│   ├── gtp_client.rs
│   ├── analysis.rs
│   ├── render_frame.rs
│   ├── ffi.rs
│   └── go_core.h       ← C header for Swift
├── tests/
│   ├── game_state_test.rs
│   ├── move_history_test.rs
│   ├── gtp_client_test.rs
│   └── render_frame_test.rs
└── target/release/libgo_core.a
```

All game logic is in Rust, callable from Swift via C ABI.
