use crate::error::GoCoreError;
use crate::render_frame;
use regex::Regex;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command};
use std::sync::{Arc, Mutex};

const ERR_BUF_CAPACITY: usize = 500;
const SUGGESTION_ANALYSIS_VISITS: u32 = 30;

pub struct GtpClient {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
    max_visits: u32,
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
        assert_eq!((suggestions[0].col, suggestions[0].row, suggestions[0].order), (3, 15, 0));
        assert_eq!((suggestions[1].col, suggestions[1].row, suggestions[1].order), (15, 3, 1));
        assert_eq!((suggestions[2].col, suggestions[2].row, suggestions[2].order), (9, 9, 2));
    }
}

impl GtpClient {
    pub fn new() -> Self {
        GtpClient {
            process: None,
            stdin: None,
            stdout: None,
            stderr_lines: Arc::new(Mutex::new(VecDeque::with_capacity(ERR_BUF_CAPACITY))),
            max_visits: 60,
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
        let stderr = child.stderr.take().ok_or(GoCoreError::PipeClosed)?;
        let stderr_lines = self.stderr_lines.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
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
        Ok(())
    }

    pub async fn play(&mut self, color: &str, vertex: &str) -> crate::error::Result<()> {
        self.command(&format!("play {color} {vertex}"), Some(60.0))
            .await?;
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
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_lines: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
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
        self.command("undo", Some(30.0)).await?;
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

    pub fn is_running(&mut self) -> bool {
        self.process
            .as_mut()
            .map(|p| matches!(p.try_wait(), Ok(None)))
            .unwrap_or(false)
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
            loop {
                let mut line = String::new();
                stdout
                    .read_line(&mut line)
                    .map_err(|_| GoCoreError::PipeClosed)?;
                if line.is_empty() {
                    return Err(GoCoreError::PipeClosed);
                }

                let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                if !saw_success {
                    if trimmed.starts_with('=') {
                        saw_success = true;
                        let payload = trimmed[1..].trim();
                        if !payload.is_empty() {
                            stdout_lines.push(payload.to_string());
                        }
                        continue;
                    }
                    if trimmed.starts_with('?') {
                        return Err(GoCoreError::Rejected(trimmed[1..].trim().to_string()));
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

        if playing_max_visits < SUGGESTION_ANALYSIS_VISITS {
            self.command(
                &format!("kata-set-param maxVisits {playing_max_visits}"),
                Some(10.0),
            )
            .await?;
        }

        suggestions_result
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

        let stdin = self.stdin.as_mut().ok_or(GoCoreError::PipeClosed)?;
        let stdout = self.stdout.as_mut().ok_or(GoCoreError::PipeClosed)?;

        let cmd_with_nl = format!("{cmd}\n");
        stdin
            .write_all(cmd_with_nl.as_bytes())
            .map_err(|_| GoCoreError::PipeClosed)?;
        stdin.flush().map_err(|_| GoCoreError::PipeClosed)?;

        let fut = async {
            let mut response = String::new();
            loop {
                let mut line = String::new();
                stdout
                    .read_line(&mut line)
                    .map_err(|_| GoCoreError::PipeClosed)?;
                if line.is_empty() {
                    return Err(GoCoreError::PipeClosed);
                }
                let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
                if trimmed.starts_with('=') {
                    response.push_str(trimmed[1..].trim());
                    return Ok::<String, GoCoreError>(response.trim().to_string());
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
                let Ok(s_re) = Regex::new(r"\bS\s+([-\d.]+)c\b") else {
                    continue;
                };
                let Some(scaps) = s_re.captures(line) else {
                    continue;
                };
                scaps[1].parse::<f64>().unwrap_or(0.0)
            };

            let visits_re = Regex::new(r"\bN\s+(\d+)\b").ok();
            let visits: u32 = visits_re
                .and_then(|r| r.captures(line))
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
        suggestions.sort_by(|a, b| b.visits.cmp(&a.visits));

        let mut unique = Vec::new();
        for suggestion in suggestions {
            if unique
                .iter()
                .any(|s: &render_frame::MoveSuggestion| s.col == suggestion.col && s.row == suggestion.row)
            {
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
