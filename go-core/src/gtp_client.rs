use crate::analysis_events;
use crate::analysis_parser::AnalysisParser;
use crate::engine_adapter::{
    EngineAdapter, EngineConfig, EngineFuture, EngineGenMove, PositionEval,
};
use crate::error::GoCoreError;
use crate::render_frame;
use regex::Regex;
use std::cmp::Reverse;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const ERR_BUF_CAPACITY: usize = 500;
const SUGGESTION_ANALYSIS_VISITS: u32 = 30;

pub struct GtpClient {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
    max_visits: u32,
    engine_config: Option<EngineConfig>,
    played_moves: Vec<(String, String)>,
}

impl GtpClient {
    pub fn new() -> Self {
        Self {
            process: None,
            stdin: None,
            stdout: None,
            stderr_lines: Arc::new(Mutex::new(VecDeque::with_capacity(ERR_BUF_CAPACITY))),
            max_visits: 60,
            engine_config: None,
            played_moves: Vec::new(),
        }
    }

    pub async fn start(
        &mut self,
        binary_path: &str,
        config_path: &str,
        model_path: &str,
        board_size: u8,
        timeout_secs: f64,
    ) -> crate::error::Result<()> {
        self.engine_config = Some(EngineConfig {
            binary_path: binary_path.to_string(),
            config_path: config_path.to_string(),
            model_path: model_path.to_string(),
            board_size,
            timeout_secs,
        });
        self.played_moves.clear();

        let mut child = Command::new(binary_path)
            .arg("gtp")
            .arg("-config")
            .arg(config_path)
            .arg("-model")
            .arg(model_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().ok_or(GoCoreError::PipeClosed)?;
        let stdout = child.stdout.take().ok_or(GoCoreError::PipeClosed)?;
        Self::set_stdout_nonblocking(&stdout)?;
        let stderr = child.stderr.take().ok_or(GoCoreError::PipeClosed)?;
        let stderr_lines = self.stderr_lines.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut analysis_parser = AnalysisParser::new(board_size);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(frame) = analysis_parser.process_line(&line) {
                    analysis_events::dispatch_analysis_frame(frame, board_size);
                }

                let mut buf = match stderr_lines.lock() {
                    Ok(buf) => buf,
                    Err(_) => break,
                };
                if buf.len() >= ERR_BUF_CAPACITY {
                    buf.pop_front();
                }
                buf.push_back(line);
            }
        });

        self.process = Some(child);
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout));
        self.command(&format!("boardsize {board_size}"), Some(timeout_secs))
            .await?;
        self.command("clear_board", Some(timeout_secs)).await?;
        self.played_moves.clear();
        Ok(())
    }

    pub async fn play(&mut self, color: &str, vertex: &str) -> crate::error::Result<()> {
        self.ensure_alive().await?;
        self.command(&format!("play {color} {vertex}"), Some(60.0))
            .await?;
        self.played_moves
            .push((color.to_string(), vertex.to_string()));
        Ok(())
    }

    pub async fn genmove(
        &mut self,
        color: &str,
        board_size: u8,
    ) -> crate::error::Result<(
        String,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Vec<render_frame::MoveSuggestion>,
    )> {
        self.clear_stderr();
        let response = self
            .command(&format!("genmove {color}"), Some(120.0))
            .await?;
        let vertex = response.trim().to_lowercase();

        let (winrate, lead, accuracy, suggestions) = {
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            let wr = Self::parse_winrate(&all_stderr);
            let ld = Self::parse_lead(&all_stderr);
            let acc = Self::parse_evaluation_accuracy(&all_stderr, self.max_visits);
            let sg = Self::parse_move_suggestions(&all_stderr, board_size);
            (wr, ld, acc, sg)
        };

        let winrate = if winrate.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            Self::parse_winrate(&all_stderr)
        } else {
            winrate
        };

        let lead = if lead.is_none() {
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            Self::parse_lead(&all_stderr)
        } else {
            lead
        };

        let accuracy = if accuracy.is_none() {
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            Self::parse_evaluation_accuracy(&all_stderr, self.max_visits)
        } else {
            accuracy
        };

        Ok((vertex, winrate, lead, accuracy, suggestions))
    }

    pub async fn final_score(&mut self) -> crate::error::Result<String> {
        let response = self.command("final_score", Some(120.0)).await?;
        Ok(response.trim().to_string())
    }

    pub async fn analyze_ownership(&mut self, board_size: u8) -> crate::error::Result<Vec<f64>> {
        if self.process.is_none() {
            return Err(GoCoreError::ProcessNotRunning);
        }
        self.clear_stderr();

        {
            let stdin = self.stdin.as_mut().ok_or(GoCoreError::PipeClosed)?;
            stdin
                .write_all(b"kata-analyze 0 ownership true\n")
                .map_err(|_| GoCoreError::PipeClosed)?;
            stdin.flush().map_err(|_| GoCoreError::PipeClosed)?;
        }

        let started_at = std::time::Instant::now();
        let expected = (board_size as usize) * (board_size as usize);
        loop {
            let all_lines: Vec<String> = {
                let stderr_lines = self
                    .stderr_lines
                    .lock()
                    .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
                stderr_lines.iter().cloned().collect()
            };
            if let Some(ownership) = Self::try_parse_ownership(&all_lines, expected) {
                let _ = self.command("name", Some(5.0)).await;
                return Ok(ownership);
            }
            if started_at.elapsed() >= std::time::Duration::from_millis(2500) {
                let _ = self.command("name", Some(5.0)).await;
                return Ok(Vec::new());
            }
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
    }

    fn try_parse_ownership(lines: &[String], expected_count: usize) -> Option<Vec<f64>> {
        for line in lines.iter().rev() {
            let trimmed = line.trim();
            if !trimmed.starts_with("ownership ") {
                continue;
            }
            let values: Vec<f64> = trimmed
                .strip_prefix("ownership ")
                .unwrap_or("")
                .split_whitespace()
                .filter_map(|s| s.parse::<f64>().ok())
                .collect();
            if values.len() == expected_count {
                return Some(values);
            }
        }
        None
    }

    #[cfg(test)]
    fn parse_ownership_from_stderr(
        lines: &[String],
        board_size: u8,
    ) -> crate::error::Result<Vec<f64>> {
        let expected = (board_size as usize) * (board_size as usize);
        if let Some(ownership) = Self::try_parse_ownership(lines, expected) {
            Ok(ownership)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn undo(&mut self) -> crate::error::Result<()> {
        self.ensure_alive().await?;
        self.command("undo", Some(30.0)).await?;
        self.played_moves.pop();
        Ok(())
    }

    pub async fn set_param(&mut self, key: &str, value: &str) -> crate::error::Result<()> {
        self.command(&format!("kata-set-param {key} {value}"), Some(10.0))
            .await?;
        if key.eq_ignore_ascii_case("maxVisits") {
            if let Ok(max_visits) = value.parse::<u32>() {
                self.max_visits = max_visits.max(1);
            }
        }
        Ok(())
    }

    pub async fn close(&mut self) -> crate::error::Result<()> {
        if let Some(stdin) = self.stdin.as_mut() {
            let _ = stdin.write_all(b"quit\n");
            let _ = stdin.flush();
        }
        if let Some(ref mut process) = self.process {
            let _ = process.wait();
        }
        self.stdin = None;
        self.stdout = None;
        self.process = None;
        Ok(())
    }

    pub async fn clear_board(&mut self, timeout: Option<f64>) -> crate::error::Result<()> {
        self.ensure_alive().await?;
        self.command("clear_board", timeout).await?;
        self.played_moves.clear();
        Ok(())
    }

    pub fn is_running(&mut self) -> bool {
        self.process
            .as_mut()
            .map(|p| matches!(p.try_wait(), Ok(None)))
            .unwrap_or(false)
    }

    async fn ensure_alive(&mut self) -> crate::error::Result<()> {
        if self.is_running() {
            return Ok(());
        }
        self.heal().await
    }

    async fn heal(&mut self) -> crate::error::Result<()> {
        self.stdin = None;
        self.stdout = None;
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.process = None;

        let config = self
            .engine_config
            .clone()
            .ok_or(GoCoreError::ProcessNotRunning)?;

        let saved_moves: Vec<(String, String)> = self.played_moves.to_vec();

        self.start(
            &config.binary_path,
            &config.config_path,
            &config.model_path,
            config.board_size,
            config.timeout_secs,
        )
        .await?;

        for (color, vertex) in &saved_moves {
            self.command(&format!("play {color} {vertex}"), Some(60.0))
                .await?;
        }
        self.played_moves = saved_moves;

        Ok(())
    }

    fn clear_stderr(&mut self) {
        if let Ok(mut buf) = self.stderr_lines.lock() {
            buf.clear();
        }
    }

    pub async fn quick_analyze(
        &mut self,
        color: &str,
        board_size: u8,
    ) -> crate::error::Result<Vec<render_frame::MoveSuggestion>> {
        if self.process.is_none() {
            return Err(GoCoreError::ProcessNotRunning);
        }
        self.clear_stderr();

        let playing_max_visits = self.max_visits;
        if playing_max_visits < SUGGESTION_ANALYSIS_VISITS {
            self.command(
                &format!("kata-set-param maxVisits {SUGGESTION_ANALYSIS_VISITS}"),
                Some(10.0),
            )
            .await?;
        }

        let suggestions_result = (|| {
            let stdin = self.stdin.as_mut().ok_or(GoCoreError::PipeClosed)?;
            let command = format!("kata-search_analyze {color} 50\n");
            stdin
                .write_all(command.as_bytes())
                .map_err(|_| GoCoreError::PipeClosed)?;
            stdin.flush().map_err(|_| GoCoreError::PipeClosed)?;

            let stdout = self.stdout.as_mut().ok_or(GoCoreError::PipeClosed)?;
            let mut stdout_lines = Vec::new();
            let mut saw_success = false;
            let deadline = Some(Instant::now() + Duration::from_secs(10));
            loop {
                let line = Self::read_stdout_line(stdout, deadline, "kata-search_analyze")?;
                if line.is_empty() {
                    return Err(GoCoreError::PipeClosed);
                }

                let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                if !saw_success {
                    if let Some(stripped) = trimmed.strip_prefix('=') {
                        saw_success = true;
                        let payload = stripped.trim();
                        if !payload.is_empty() {
                            stdout_lines.push(payload.to_string());
                        }
                        continue;
                    }
                    if let Some(stripped) = trimmed.strip_prefix('?') {
                        return Err(GoCoreError::Rejected(stripped.trim().to_string()));
                    }
                    stdout_lines.push(trimmed.to_string());
                    continue;
                }

                if trimmed.is_empty() {
                    break;
                }
                stdout_lines.push(trimmed.to_string());
            }

            let started_at = std::time::Instant::now();
            loop {
                let stderr_lines = self
                    .stderr_lines
                    .lock()
                    .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
                let mut analysis_lines: Vec<String> = stderr_lines.iter().cloned().collect();
                drop(stderr_lines);
                analysis_lines.extend(stdout_lines.clone());

                let suggestions = Self::parse_move_suggestions(&analysis_lines, board_size);
                if !suggestions.is_empty()
                    || started_at.elapsed() >= std::time::Duration::from_millis(800)
                {
                    return Ok(suggestions);
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        })();

        if matches!(suggestions_result, Err(GoCoreError::Timeout(_))) {
            self.kill_process();
            return suggestions_result;
        }

        if playing_max_visits < SUGGESTION_ANALYSIS_VISITS {
            self.command(
                &format!("kata-set-param maxVisits {playing_max_visits}"),
                Some(10.0),
            )
            .await?;
        }

        suggestions_result
    }

    pub async fn evaluate_position(
        &mut self,
        color: &str,
        board_size: u8,
        num_visits: i32,
    ) -> crate::error::Result<PositionEval> {
        if self.process.is_none() {
            return Err(GoCoreError::ProcessNotRunning);
        }
        self.clear_stderr();

        let saved_visits = self.max_visits;
        let target_visits = (num_visits.max(1) as u32).min(5000);
        if saved_visits != target_visits {
            self.command(
                &format!("kata-set-param maxVisits {target_visits}"),
                Some(10.0),
            )
            .await?;
            self.max_visits = target_visits;
        }

        let eval_result = (|| -> crate::error::Result<PositionEval> {
            {
                let stdin = self.stdin.as_mut().ok_or(GoCoreError::PipeClosed)?;
                let cmd = format!("kata-search_analyze {color} 10\n");
                stdin
                    .write_all(cmd.as_bytes())
                    .map_err(|_| GoCoreError::PipeClosed)?;
                stdin.flush().map_err(|_| GoCoreError::PipeClosed)?;
            }

            {
                let stdout = self.stdout.as_mut().ok_or(GoCoreError::PipeClosed)?;
                let deadline = Some(Instant::now() + Duration::from_secs(10));
                let mut saw_success = false;
                loop {
                    let line = Self::read_stdout_line(stdout, deadline, "kata-search_analyze")?;
                    if line.is_empty() {
                        return Err(GoCoreError::PipeClosed);
                    }
                    let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                    if !saw_success {
                        if trimmed.starts_with('=') {
                            saw_success = true;
                            continue;
                        }
                        if trimmed.starts_with('?') {
                            let msg = trimmed.strip_prefix('?').unwrap_or("").trim().to_string();
                            return Err(GoCoreError::Rejected(msg));
                        }
                        continue;
                    }
                    if trimmed.is_empty() {
                        break;
                    }
                }
            }

            let started_at = Instant::now();
            loop {
                let lines: Vec<String> = {
                    let buf = self
                        .stderr_lines
                        .lock()
                        .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
                    buf.iter().cloned().collect()
                };

                let wr = Self::parse_winrate(&lines);
                let ld = Self::parse_lead(&lines);
                let sg = Self::parse_move_suggestions(&lines, board_size);

                if wr.is_some()
                    || !sg.is_empty()
                    || started_at.elapsed() >= Duration::from_millis(1500)
                {
                    return Ok(PositionEval {
                        winrate_black: wr.unwrap_or(0.5),
                        lead_black: ld.unwrap_or(0.0),
                        move_suggestions: sg,
                    });
                }

                std::thread::sleep(Duration::from_millis(30));
            }
        })();

        if saved_visits != target_visits {
            self.max_visits = saved_visits;
            let _ = self
                .command(
                    &format!("kata-set-param maxVisits {saved_visits}"),
                    Some(10.0),
                )
                .await;
        }

        eval_result
    }

    // ── Low-level command ──────────────────────────────────

    pub async fn command(
        &mut self,
        cmd: &str,
        timeout: Option<f64>,
    ) -> crate::error::Result<String> {
        if self.process.is_none() {
            return Err(GoCoreError::ProcessNotRunning);
        }

        let cmd_with_nl = format!("{cmd}\n");
        {
            let stdin = self.stdin.as_mut().ok_or(GoCoreError::PipeClosed)?;
            stdin
                .write_all(cmd_with_nl.as_bytes())
                .map_err(|_| GoCoreError::PipeClosed)?;
            stdin.flush().map_err(|_| GoCoreError::PipeClosed)?;
        }

        let deadline = timeout.map(|t| Instant::now() + Duration::from_secs_f64(t));
        let mut response = String::new();
        let result = loop {
            let stdout = self.stdout.as_mut().ok_or(GoCoreError::PipeClosed)?;
            let line = match Self::read_stdout_line(stdout, deadline, cmd) {
                Ok(line) => line,
                Err(e) => break Err(e),
            };
            if line.is_empty() {
                break Err(GoCoreError::PipeClosed);
            }
            let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
            if let Some(stripped) = trimmed.strip_prefix('=') {
                response.push_str(stripped.trim());
                break Ok(response.trim().to_string());
            } else if let Some(stripped) = trimmed.strip_prefix('?') {
                let msg = stripped.trim();
                break Err(GoCoreError::Rejected(msg.to_string()));
            } else {
                response.push_str(trimmed);
                response.push('\n');
            }
        };

        if matches!(result, Err(GoCoreError::Timeout(_))) {
            self.kill_process();
        }

        result
    }

    fn kill_process(&mut self) {
        self.stdin = None;
        self.stdout = None;
        if let Some(process) = self.process.as_mut() {
            let _ = process.kill();
            let _ = process.wait();
        }
        self.process = None;
    }

    fn read_stdout_line(
        stdout: &mut BufReader<ChildStdout>,
        deadline: Option<Instant>,
        command_name: &str,
    ) -> crate::error::Result<String> {
        loop {
            let mut line = String::new();
            match stdout.read_line(&mut line) {
                Ok(_) => return Ok(line),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                        return Err(GoCoreError::Timeout(command_name.to_string()));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(GoCoreError::Io(e)),
            }
        }
    }

    #[cfg(unix)]
    fn set_stdout_nonblocking(stdout: &ChildStdout) -> crate::error::Result<()> {
        use std::os::fd::AsRawFd;

        let fd = stdout.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(GoCoreError::Io(std::io::Error::last_os_error()));
        }

        let result = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if result < 0 {
            return Err(GoCoreError::Io(std::io::Error::last_os_error()));
        }

        Ok(())
    }

    #[cfg(not(unix))]
    fn set_stdout_nonblocking(_stdout: &ChildStdout) -> crate::error::Result<()> {
        Ok(())
    }

    // ── Stderr parsing ─────────────────────────────────────

    fn parse_winrate(lines: &[String]) -> Option<f64> {
        let winrate_re = Regex::new(r"^\s*:\s+.*\bW\s+([-\d.]+)c\b").ok()?;

        for line in lines.iter().rev() {
            if let Some(caps) = winrate_re.captures(line) {
                let wr: f64 = caps[1].parse().ok()?;
                return Some((wr / 100.0).clamp(0.0, 1.0));
            }
        }
        None
    }

    fn parse_lead(lines: &[String]) -> Option<f64> {
        if let Some(score_lead) = Self::parse_score_lead(lines) {
            return Some(score_lead);
        }

        let lead_re = Regex::new(r"^\s*:\s+.*\bS\s+([-\d.]+)c\b").ok()?;
        for line in lines.iter().rev() {
            if let Some(caps) = lead_re.captures(line) {
                return caps[1].parse().ok();
            }
        }
        None
    }

    fn parse_score_lead(lines: &[String]) -> Option<f64> {
        let score_re = Regex::new(r"^\s*:\s+.*\(\s*([+-]?\d+(?:\.\d+)?)\s+L\b").ok()?;
        for line in lines.iter().rev() {
            if let Some(caps) = score_re.captures(line) {
                return caps[1].parse().ok();
            }
        }
        None
    }

    fn parse_evaluation_accuracy(lines: &[String], max_visits: u32) -> Option<f64> {
        let visits_re = Regex::new(r"\bRoot visits:\s+(\d+)\b").ok()?;
        let max_visits = max_visits.max(1) as f64;
        for line in lines.iter().rev() {
            if let Some(caps) = visits_re.captures(line) {
                let visits: f64 = caps[1].parse().ok()?;
                return Some((visits / max_visits).clamp(0.0, 1.0));
            }
        }
        None
    }

    fn parse_move_suggestions(
        lines: &[String],
        board_size: u8,
    ) -> Vec<render_frame::MoveSuggestion> {
        let suggestions = Self::parse_info_move_suggestions(lines, board_size);
        if !suggestions.is_empty() {
            return Self::top_unique_suggestions(suggestions);
        }

        let Ok(suggestion_re) = Regex::new(r"^([A-Z]\d+)\s*:\s+T\s+[-\d.]+c\s+W\s+([-\d.]+)c")
        else {
            return vec![];
        };
        let Ok(lead_re) = Regex::new(r"\(\s*([+-]?\d+(?:\.\d+)?)\s+L\b") else {
            return vec![];
        };
        let Ok(score_re) = Regex::new(r"\bS\s+([-\d.]+)c\b") else {
            return vec![];
        };
        let Ok(visits_re) = Regex::new(r"\bN\s+(\d+)\b") else {
            return vec![];
        };
        let cols = b"ABCDEFGHJKLMNOPQRST";

        let mut suggestions: Vec<render_frame::MoveSuggestion> = Vec::new();

        for line in lines.iter() {
            let Some(caps) = suggestion_re.captures(line) else {
                continue;
            };
            let vertex = caps[1].to_string();
            if vertex.eq_ignore_ascii_case("PASS") || vertex.eq_ignore_ascii_case("RESIGN") {
                continue;
            }

            let Ok(winrate_centi) = caps[2].parse::<f64>() else {
                continue;
            };
            let winrate = (winrate_centi / 100.0).clamp(0.0, 1.0);

            let lead: f64 = if let Some(lcaps) = lead_re.captures(line) {
                lcaps[1].parse().unwrap_or(0.0)
            } else {
                let Some(scaps) = score_re.captures(line) else {
                    continue;
                };
                scaps[1].parse::<f64>().unwrap_or(0.0)
            };

            let visits: u32 = visits_re
                .captures(line)
                .and_then(|c| c[1].parse().ok())
                .unwrap_or(0);

            let bytes = vertex.as_bytes();
            let col_char = bytes[0];
            let Some(col) = cols.iter().position(|&c| c == col_char) else {
                continue;
            };
            let col = col as u8;
            let Ok(row_num) = vertex[1..].parse::<u8>() else {
                continue;
            };
            let row = board_size.saturating_sub(row_num);
            if col >= board_size || row >= board_size {
                continue;
            }

            suggestions.push(render_frame::MoveSuggestion {
                col,
                row,
                winrate,
                lead,
                visits,
                order: 0,
            });
        }

        Self::top_unique_suggestions(suggestions)
    }

    fn parse_info_move_suggestions(
        lines: &[String],
        board_size: u8,
    ) -> Vec<render_frame::MoveSuggestion> {
        let Ok(info_re) = Regex::new(
            r"\binfo move ([A-Z]\d+|PASS|RESIGN|pass|resign)\s+visits\s+(\d+).*?\bwinrate\s+([-\d.]+).*?\bscoreLead\s+([-\d.]+).*?\border\s+(\d+)",
        ) else {
            return vec![];
        };
        let cols = b"ABCDEFGHJKLMNOPQRST";
        let mut suggestions = Vec::new();

        for line in lines {
            for caps in info_re.captures_iter(line) {
                let vertex = caps[1].to_string();
                if vertex.eq_ignore_ascii_case("PASS") || vertex.eq_ignore_ascii_case("RESIGN") {
                    continue;
                }

                let Ok(visits) = caps[2].parse::<u32>() else {
                    continue;
                };
                let Ok(winrate_raw) = caps[3].parse::<f64>() else {
                    continue;
                };
                let winrate = if winrate_raw > 1.0 {
                    (winrate_raw / 10000.0).clamp(0.0, 1.0)
                } else {
                    winrate_raw.clamp(0.0, 1.0)
                };
                let lead = caps[4].parse::<f64>().unwrap_or(0.0);
                let order = caps[5].parse::<u32>().unwrap_or(0);

                let bytes = vertex.as_bytes();
                let col_char = bytes[0];
                let Some(col) = cols.iter().position(|&c| c == col_char) else {
                    continue;
                };
                let col = col as u8;
                let Ok(row_num) = vertex[1..].parse::<u8>() else {
                    continue;
                };
                let row = board_size.saturating_sub(row_num);
                if col >= board_size || row >= board_size {
                    continue;
                }

                suggestions.push(render_frame::MoveSuggestion {
                    col,
                    row,
                    winrate,
                    lead,
                    visits,
                    order,
                });
            }
        }

        suggestions
    }

    fn top_unique_suggestions(
        mut suggestions: Vec<render_frame::MoveSuggestion>,
    ) -> Vec<render_frame::MoveSuggestion> {
        suggestions.sort_by_key(|suggestion| Reverse(suggestion.visits));

        let mut unique = Vec::new();
        for suggestion in suggestions {
            if unique.iter().any(|s: &render_frame::MoveSuggestion| {
                s.col == suggestion.col && s.row == suggestion.row
            }) {
                continue;
            }
            unique.push(suggestion);
            if unique.len() == 3 {
                break;
            }
        }

        for (i, suggestion) in unique.iter_mut().enumerate() {
            suggestion.order = i as u32;
        }
        unique
    }
}

impl Drop for GtpClient {
    fn drop(&mut self) {
        if let Some(ref mut stdin) = self.stdin {
            let _ = stdin.write_all(b"quit\n");
            let _ = stdin.flush();
        }
        std::thread::sleep(Duration::from_millis(200));
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Default for GtpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineAdapter for GtpClient {
    fn start<'a>(&'a mut self, config: EngineConfig) -> EngineFuture<'a, ()> {
        Box::pin(async move {
            self.start(
                &config.binary_path,
                &config.config_path,
                &config.model_path,
                config.board_size,
                config.timeout_secs,
            )
            .await
        })
    }

    fn close<'a>(&'a mut self) -> EngineFuture<'a, ()> {
        Box::pin(async move { self.close().await })
    }

    fn play<'a>(&'a mut self, color: &'a str, vertex: &'a str) -> EngineFuture<'a, ()> {
        Box::pin(async move { self.play(color, vertex).await })
    }

    fn genmove<'a>(
        &'a mut self,
        color: &'a str,
        board_size: u8,
    ) -> EngineFuture<'a, EngineGenMove> {
        Box::pin(async move {
            let (vertex, winrate_black, lead_black, evaluation_accuracy, suggestions) =
                self.genmove(color, board_size).await?;
            Ok(EngineGenMove {
                vertex,
                winrate_black,
                lead_black,
                evaluation_accuracy,
                suggestions,
            })
        })
    }

    fn undo<'a>(&'a mut self) -> EngineFuture<'a, ()> {
        Box::pin(async move { self.undo().await })
    }

    fn final_score<'a>(&'a mut self) -> EngineFuture<'a, String> {
        Box::pin(async move { self.final_score().await })
    }

    fn quick_analyze<'a>(
        &'a mut self,
        color: &'a str,
        board_size: u8,
    ) -> EngineFuture<'a, Vec<render_frame::MoveSuggestion>> {
        Box::pin(async move { self.quick_analyze(color, board_size).await })
    }

    fn analyze_ownership<'a>(&'a mut self, board_size: u8) -> EngineFuture<'a, Vec<f64>> {
        Box::pin(async move { self.analyze_ownership(board_size).await })
    }

    fn evaluate_position<'a>(
        &'a mut self,
        color: &'a str,
        board_size: u8,
        num_visits: i32,
    ) -> EngineFuture<'a, PositionEval> {
        Box::pin(async move { self.evaluate_position(color, board_size, num_visits).await })
    }

    fn command<'a>(
        &'a mut self,
        command: &'a str,
        timeout: Option<f64>,
    ) -> EngineFuture<'a, String> {
        Box::pin(async move { self.command(command, timeout).await })
    }

    fn clear_board<'a>(&'a mut self, timeout: Option<f64>) -> EngineFuture<'a, ()> {
        Box::pin(async move { self.clear_board(timeout).await })
    }

    fn is_running(&mut self) -> bool {
        self.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::GtpClient;

    #[test]
    fn parses_root_winrate_as_black_perspective() {
        let lines = vec![
            "---White(^)---".to_string(),
            ": T  83.53c W  64.96c S   5.51c ( +3.5 L  +3.5) N      64  --  D4 D5 pass pass".to_string(),
            "D4  : T  96.86c W  77.33c S  10.64c ( +5.2 L  +5.2) LCB   38.50c P  0.75% WF 109.9 PSV     102 N      29  --  D4 D5 pass pass".to_string(),
        ];

        let winrate = GtpClient::parse_winrate(&lines).unwrap();
        let lead = GtpClient::parse_lead(&lines).unwrap();

        assert!((winrate - 0.6496).abs() < 0.0001);
        assert!((lead - 3.5).abs() < 0.0001);
    }

    #[test]
    fn bundled_gtp_config_reports_analysis_from_black_perspective() {
        let config = include_str!("../../kata-engine/gtp.cfg");
        assert!(config.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && trimmed == "reportAnalysisWinratesAs = BLACK"
        }));
    }

    #[test]
    fn clamps_root_winrate_to_percentage_bounds() {
        let lines = vec![
            ": T  120.00c W  101.20c S  40.00c (+22.0 L +22.0) N      64  --  pass".to_string(),
        ];

        let winrate = GtpClient::parse_winrate(&lines).unwrap();

        assert_eq!(winrate, 1.0);
    }

    #[test]
    fn parses_evaluation_accuracy_from_root_visits() {
        let lines = vec![
            "Root visits: 64".to_string(),
            "New playouts: 64".to_string(),
        ];

        let accuracy = GtpClient::parse_evaluation_accuracy(&lines, 120).unwrap();

        assert!((accuracy - 64.0 / 120.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_candidate_move_suggestions() {
        let lines = vec![
            ": T  83.53c W  64.96c S   5.51c ( +3.5 L  +3.5) N      64  --  D4 D5 pass pass".to_string(),
            "D4  : T  96.86c W  77.33c S  10.64c ( +5.2 L  +5.2) LCB   38.50c P  0.75% WF 109.9 PSV     102 N      29  --  D4 D5 pass pass".to_string(),
            "Q16 : T  82.30c W  63.12c S   4.89c ( +2.8 L  +2.8) N      18  --  Q16 R15 pass pass".to_string(),
        ];

        let suggestions = GtpClient::parse_move_suggestions(&lines, 19);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].order, 0);
        assert_eq!(suggestions[0].col, 3);
        assert_eq!(suggestions[0].row, 15);
        assert!((suggestions[0].winrate - 0.7733).abs() < 0.0001);
        assert!((suggestions[0].lead - 5.2).abs() < 0.001);
        assert_eq!(suggestions[0].visits, 29);

        assert_eq!(suggestions[1].order, 1);
        assert_eq!(suggestions[1].col, 15);
        assert_eq!(suggestions[1].row, 3);
    }

    #[test]
    fn parse_move_suggestions_returns_empty_when_no_candidates() {
        let lines = vec![
            ": T  83.53c W  64.96c S   5.51c ( +3.5 L  +3.5) N      64  --  D4 D5 pass pass"
                .to_string(),
        ];
        let suggestions = GtpClient::parse_move_suggestions(&lines, 19);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn move_suggestions_keep_top_three_unique_points() {
        let lines = vec![
            "info move D4 visits 40 edgeVisits 40 utility 0 winrate 0.55 scoreLead 2.1 order 0 pv D4 Q16 info move D4 visits 36 edgeVisits 36 utility 0 winrate 0.54 scoreLead 1.9 order 1 pv D4 Q4 info move Q16 visits 30 edgeVisits 30 utility 0 winrate 0.53 scoreLead 1.7 order 2 pv Q16 D4 info move K10 visits 20 edgeVisits 20 utility 0 winrate 0.52 scoreLead 1.2 order 3 pv K10 D4 info move Q4 visits 10 edgeVisits 10 utility 0 winrate 0.51 scoreLead 0.8 order 4 pv Q4 D4".to_string(),
        ];

        let suggestions = GtpClient::parse_move_suggestions(&lines, 19);

        assert_eq!(suggestions.len(), 3);
        assert_eq!(
            (suggestions[0].col, suggestions[0].row, suggestions[0].order),
            (3, 15, 0)
        );
        assert_eq!(
            (suggestions[1].col, suggestions[1].row, suggestions[1].order),
            (15, 3, 1)
        );
        assert_eq!(
            (suggestions[2].col, suggestions[2].row, suggestions[2].order),
            (9, 9, 2)
        );
    }

    #[test]
    fn parses_ownership_from_stderr_snapshot() {
        let lines = vec![
            "info unrelated".to_string(),
            "ownership 0.1 -0.2 0.3 -0.4".to_string(),
        ];

        let ownership = GtpClient::parse_ownership_from_stderr(&lines, 2).unwrap();

        assert_eq!(ownership, vec![0.1, -0.2, 0.3, -0.4]);
    }
}
