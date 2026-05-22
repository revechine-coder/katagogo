use once_cell::sync::Lazy;
use regex::Regex;
use std::time::{Duration, Instant};

const DEFAULT_EMIT_INTERVAL: Duration = Duration::from_millis(100);
const GTP_COLUMNS: &[u8] = b"ABCDEFGHJKLMNOPQRST";

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateAnalysis {
    pub coordinate: (u8, u8),
    pub visits: u32,
    pub winrate: f32,
    pub score_lead: f32,
    pub pv: Vec<(u8, u8)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GtpAnalysisFrame {
    pub candidates: Vec<CandidateAnalysis>,
}

#[derive(Debug)]
pub struct AnalysisParser {
    board_size: u8,
    min_emit_interval: Duration,
    last_emit_at: Option<Instant>,
}

impl AnalysisParser {
    pub fn new(board_size: u8) -> Self {
        Self {
            board_size,
            min_emit_interval: DEFAULT_EMIT_INTERVAL,
            last_emit_at: None,
        }
    }

    pub fn with_min_emit_interval(board_size: u8, min_emit_interval: Duration) -> Self {
        Self {
            board_size,
            min_emit_interval,
            last_emit_at: None,
        }
    }

    pub fn parse_line(&self, line: &str) -> Option<GtpAnalysisFrame> {
        let candidate = parse_info_move_line(line, self.board_size)
            .or_else(|| parse_legacy_candidate_line(line, self.board_size))?;
        Some(GtpAnalysisFrame {
            candidates: vec![candidate],
        })
    }

    pub fn process_line(&mut self, line: &str) -> Option<GtpAnalysisFrame> {
        let frame = self.parse_line(line)?;
        let now = Instant::now();
        if self
            .last_emit_at
            .is_some_and(|last| now.duration_since(last) < self.min_emit_interval)
        {
            return None;
        }
        self.last_emit_at = Some(now);
        Some(frame)
    }

    pub fn parse_lines(&self, lines: &[String]) -> Option<GtpAnalysisFrame> {
        let candidates = lines
            .iter()
            .filter_map(|line| self.parse_line(line))
            .flat_map(|frame| frame.candidates)
            .collect::<Vec<_>>();

        if candidates.is_empty() {
            None
        } else {
            Some(GtpAnalysisFrame { candidates })
        }
    }
}

impl Default for AnalysisParser {
    fn default() -> Self {
        Self::new(19)
    }
}

static INFO_MOVE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\binfo move ([A-Z]\d+|PASS|RESIGN|pass|resign)\s+visits\s+(\d+).*?\bwinrate\s+([-\d.]+).*?\bscoreLead\s+([-\d.]+)(?:.*?\bpv\s+(.+))?",
    )
    .expect("valid info move regex")
});

static LEGACY_CANDIDATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*([A-Z]\d+)\s*:\s+.*?\bW\s+([-\d.]+)c\b.*?\b(?:S|L)\s+([-\d.]+)c\b.*?\bN\s+(\d+)\b(?:\s+--\s+(.+))?")
        .expect("valid legacy candidate regex")
});

fn parse_info_move_line(line: &str, board_size: u8) -> Option<CandidateAnalysis> {
    let caps = INFO_MOVE_RE.captures(line)?;
    let vertex = caps.get(1)?.as_str();
    let coordinate = parse_gtp_coordinate(vertex, board_size)?;
    let visits = caps.get(2)?.as_str().parse::<u32>().ok()?;
    let winrate = normalize_winrate(caps.get(3)?.as_str().parse::<f32>().ok()?);
    let score_lead = caps.get(4)?.as_str().parse::<f32>().ok()?;
    let pv = caps
        .get(5)
        .map(|matched| parse_pv(matched.as_str(), board_size))
        .unwrap_or_default();

    Some(CandidateAnalysis {
        coordinate,
        visits,
        winrate,
        score_lead,
        pv,
    })
}

fn parse_legacy_candidate_line(line: &str, board_size: u8) -> Option<CandidateAnalysis> {
    let caps = LEGACY_CANDIDATE_RE.captures(line)?;
    let coordinate = parse_gtp_coordinate(caps.get(1)?.as_str(), board_size)?;
    let winrate = (caps.get(2)?.as_str().parse::<f32>().ok()? / 100.0).clamp(0.0, 1.0);
    let score_lead = caps.get(3)?.as_str().parse::<f32>().ok()?;
    let visits = caps.get(4)?.as_str().parse::<u32>().ok()?;
    let pv = caps
        .get(5)
        .map(|matched| parse_pv(matched.as_str(), board_size))
        .unwrap_or_default();

    Some(CandidateAnalysis {
        coordinate,
        visits,
        winrate,
        score_lead,
        pv,
    })
}

fn normalize_winrate(raw: f32) -> f32 {
    if raw > 1.0 {
        (raw / 10000.0).clamp(0.0, 1.0)
    } else {
        raw.clamp(0.0, 1.0)
    }
}

fn parse_pv(raw: &str, board_size: u8) -> Vec<(u8, u8)> {
    raw.split_whitespace()
        .filter_map(|token| parse_gtp_coordinate(token, board_size))
        .collect()
}

pub fn parse_gtp_coordinate(vertex: &str, board_size: u8) -> Option<(u8, u8)> {
    let vertex = vertex.trim().to_ascii_uppercase();
    if vertex == "PASS" || vertex == "RESIGN" {
        return None;
    }

    let bytes = vertex.as_bytes();
    if bytes.len() < 2 {
        return None;
    }

    let col = GTP_COLUMNS.iter().position(|&c| c == bytes[0])? as u8;
    let row_num = vertex[1..].parse::<u8>().ok()?;
    let row = board_size.checked_sub(row_num)?;

    if col >= board_size || row >= board_size {
        None
    } else {
        Some((col, row))
    }
}
