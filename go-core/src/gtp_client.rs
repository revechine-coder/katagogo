use crate::error::GoCoreError;
use regex::Regex;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command};
use std::sync::{Arc, Mutex};

const ERR_BUF_CAPACITY: usize = 500;

pub struct GtpClient {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    stderr_lines: Arc<Mutex<VecDeque<String>>>,
}

impl GtpClient {
    pub fn new() -> Self {
        GtpClient {
            process: None,
            stdin: None,
            stdout: None,
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
    ) -> crate::error::Result<(String, Option<f64>, Option<f64>)> {
        let response = self.command(&format!("genmove {color}"), Some(120.0)).await?;
        let vertex = response.trim().to_lowercase();

        let (winrate, lead) = {
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            let wr = Self::parse_winrate(&all_stderr, &vertex, color);
            let ld = Self::parse_lead(&all_stderr, color);
            (wr, ld)
        };

        // Retry once with brief delay if winrate not found
        let winrate = if winrate.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let stderr_lines = self
                .stderr_lines
                .lock()
                .map_err(|_| GoCoreError::Internal("stderr lock poisoned".to_string()))?;
            let all_stderr: Vec<String> = stderr_lines.iter().cloned().collect();
            drop(stderr_lines);
            Self::parse_winrate(&all_stderr, &vertex, color)
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
            Self::parse_lead(&all_stderr, color)
        } else {
            lead
        };

        Ok((vertex, winrate, lead))
    }

    pub async fn final_score(&mut self) -> crate::error::Result<String> {
        let response = self.command("final_score", Some(120.0)).await?;
        Ok(response.trim().to_string())
    }

    pub async fn set_param(&mut self, key: &str, value: &str) -> crate::error::Result<()> {
        self.command(&format!("kata-set-param {key} {value}"), Some(10.0))
            .await?;
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
            .map(|p| p.try_wait().ok().is_none())
            .unwrap_or(false)
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

    fn parse_winrate(lines: &[String], vertex: &str, color: &str) -> Option<f64> {
        let vertex_upper = vertex.to_uppercase();
        let winrate_re = Regex::new(r"\bP\s+(\d+\.\d+)%").ok()?;

        for line in lines.iter().rev() {
            if !line.to_uppercase().contains(&vertex_upper) {
                continue;
            }
            if let Some(caps) = winrate_re.captures(line) {
                let wr: f64 = caps[1].parse().ok()?;
                let wr = wr / 100.0;
                return if color.eq_ignore_ascii_case("w") || color.eq_ignore_ascii_case("white") {
                    Some(1.0 - wr)
                } else {
                    Some(wr)
                };
            }
        }
        None
    }

    fn parse_lead(lines: &[String], color: &str) -> Option<f64> {
        let lead_re = Regex::new(r"\bS\s+([-\d.]+)c\b").ok()?;
        for line in lines.iter().rev() {
            if let Some(caps) = lead_re.captures(line) {
                let val: f64 = caps[1].parse().ok()?;
                let val = val / 100.0;
                return if color.eq_ignore_ascii_case("w") || color.eq_ignore_ascii_case("white") {
                    Some(-val)
                } else {
                    Some(val)
                };
            }
        }
        None
    }
}
