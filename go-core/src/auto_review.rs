use crate::engine_adapter::EngineAdapter;
use crate::error::Result;
use crate::game_state::{Color, MoveRecord};
use crate::render_frame::MoveSuggestion;

const BAD_MOVE_WINRATE_THRESHOLD: f64 = 0.08;
const BAD_MOVE_SCORE_THRESHOLD: f64 = 1.5;
const SLACK_WINRATE_THRESHOLD: f64 = 0.03;
const SLACK_SCORE_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveQuality {
    Good,
    BadMove,
    SlackMove,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewedMove {
    pub move_number: u32,
    pub color: String,
    pub vertex: String,
    pub winrate_before: f64,
    pub winrate_after: f64,
    pub score_before: f64,
    pub score_after: f64,
    pub winrate_drop: f64,
    pub score_drop: f64,
    pub quality: MoveQuality,
    pub ai_best_move: Option<(u8, u8)>,
    pub top_suggestions: Vec<MoveSuggestion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutoReview {
    pub moves: Vec<ReviewedMove>,
    pub total_moves: u32,
    pub bad_move_count: u32,
    pub slack_move_count: u32,
}

impl AutoReview {
    pub fn empty() -> Self {
        AutoReview {
            moves: Vec::new(),
            total_moves: 0,
            bad_move_count: 0,
            slack_move_count: 0,
        }
    }

    pub async fn run_scan(
        engine: &mut dyn EngineAdapter,
        records: &[MoveRecord],
        board_size: u8,
        num_visits: i32,
    ) -> Result<Self> {
        if records.is_empty() {
            return Ok(AutoReview::empty());
        }

        engine.clear_board(Some(30.0)).await?;

        let first_color = records[0].color.as_str();
        let mut prev_eval = engine
            .evaluate_position(first_color, board_size, num_visits)
            .await?;

        let mut reviewed_moves: Vec<ReviewedMove> = Vec::with_capacity(records.len());

        for record in records {
            let winrate_before = prev_eval.winrate_black;
            let score_before = prev_eval.lead_black;
            let suggestions_before = std::mem::take(&mut prev_eval.move_suggestions);

            engine.play(record.color.as_str(), &record.vertex).await?;

            let next_color = record.color.opponent().as_str();
            prev_eval = engine
                .evaluate_position(next_color, board_size, num_visits)
                .await?;

            let winrate_after = prev_eval.winrate_black;
            let score_after = prev_eval.lead_black;

            let (winrate_drop, score_drop) = match record.color {
                Color::Black => (winrate_before - winrate_after, score_before - score_after),
                Color::White => (winrate_after - winrate_before, score_after - score_before),
            };

            reviewed_moves.push(ReviewedMove {
                move_number: record.move_number,
                color: record.color.as_str().to_string(),
                vertex: record.vertex.clone(),
                winrate_before,
                winrate_after,
                score_before,
                score_after,
                winrate_drop,
                score_drop,
                quality: classify(winrate_drop, score_drop),
                ai_best_move: suggestions_before.first().map(|s| (s.col, s.row)),
                top_suggestions: suggestions_before,
            });
        }

        let bad_move_count = reviewed_moves
            .iter()
            .filter(|m| m.quality == MoveQuality::BadMove)
            .count() as u32;
        let slack_move_count = reviewed_moves
            .iter()
            .filter(|m| m.quality == MoveQuality::SlackMove)
            .count() as u32;

        Ok(AutoReview {
            total_moves: records.len() as u32,
            moves: reviewed_moves,
            bad_move_count,
            slack_move_count,
        })
    }
}

pub fn classify(winrate_drop: f64, score_drop: f64) -> MoveQuality {
    if winrate_drop > BAD_MOVE_WINRATE_THRESHOLD || score_drop > BAD_MOVE_SCORE_THRESHOLD {
        return MoveQuality::BadMove;
    }
    if winrate_drop > SLACK_WINRATE_THRESHOLD || score_drop > SLACK_SCORE_THRESHOLD {
        return MoveQuality::SlackMove;
    }
    MoveQuality::Good
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_bad_move_by_winrate_drop() {
        assert_eq!(classify(0.09, 0.0), MoveQuality::BadMove);
    }

    #[test]
    fn classify_bad_move_by_score_drop() {
        assert_eq!(classify(0.0, 2.0), MoveQuality::BadMove);
    }

    #[test]
    fn classify_slack_move_by_winrate_drop() {
        assert_eq!(classify(0.05, 0.0), MoveQuality::SlackMove);
    }

    #[test]
    fn classify_slack_move_by_score_drop() {
        assert_eq!(classify(0.0, 1.0), MoveQuality::SlackMove);
    }

    #[test]
    fn classify_good_move() {
        assert_eq!(classify(0.01, 0.1), MoveQuality::Good);
    }

    #[test]
    fn classify_good_move_when_winrate_improves() {
        assert_eq!(classify(-0.05, -1.0), MoveQuality::Good);
    }
}
