use crate::auto_review::{self, AutoReview, MoveQuality};
use crate::engine_adapter::{EngineAdapter, EngineConfig, MockEngineAdapter, PositionEval};
use crate::game_state::{Color, MoveRecord};
use crate::render_frame::MoveSuggestion;

fn mock_record(number: u32, color: Color, vertex: &str) -> MoveRecord {
    MoveRecord {
        move_number: number,
        color,
        vertex: vertex.to_string(),
        captured: Vec::new(),
        winrate: None,
        lead: None,
        evaluation_accuracy: None,
    }
}

fn mock_eval(
    winrate_black: f64,
    lead_black: f64,
    suggestions: Vec<MoveSuggestion>,
) -> PositionEval {
    PositionEval {
        winrate_black,
        lead_black,
        move_suggestions: suggestions,
    }
}

fn mock_suggestion(col: u8, row: u8, winrate: f64, visits: u32) -> MoveSuggestion {
    MoveSuggestion {
        col,
        row,
        winrate,
        lead: 0.0,
        visits,
        order: 0,
    }
}

async fn started_mock_engine() -> MockEngineAdapter {
    let mut engine = MockEngineAdapter::new();
    engine
        .start(EngineConfig {
            binary_path: "mock".to_string(),
            config_path: "mock.cfg".to_string(),
            model_path: "mock.bin.gz".to_string(),
            board_size: 19,
            timeout_secs: 1.0,
        })
        .await
        .unwrap();
    engine
}

#[tokio::test]
async fn run_scan_empty_records() {
    let mut engine = started_mock_engine().await;
    let review = AutoReview::run_scan(&mut engine, &[], 19, 50)
        .await
        .unwrap();
    assert_eq!(review.total_moves, 0);
    assert!(review.moves.is_empty());
}

#[tokio::test]
async fn run_scan_single_good_move() {
    let records = vec![mock_record(1, Color::Black, "D4")];
    let mut engine = started_mock_engine().await;

    // Initial position eval (Black's turn, 50% winrate, 0 lead)
    engine.push_evaluation(mock_eval(0.50, 0.0, vec![]));
    // After Black plays D4 (still ~50%)
    engine.push_evaluation(mock_eval(0.50, 0.0, vec![]));

    let review = AutoReview::run_scan(&mut engine, &records, 19, 50)
        .await
        .unwrap();

    assert_eq!(review.total_moves, 1);
    assert_eq!(review.bad_move_count, 0);
    assert_eq!(review.slack_move_count, 0);
    assert_eq!(review.moves[0].quality, MoveQuality::Good);
    assert_eq!(review.moves[0].vertex, "D4");
}

#[tokio::test]
async fn run_scan_detects_bad_move_by_winrate_drop() {
    let records = vec![mock_record(1, Color::Black, "Q16")];
    let mut engine = started_mock_engine().await;

    // Initial: Black has 65% winrate at this position
    engine.push_evaluation(mock_eval(0.65, 5.0, vec![mock_suggestion(3, 3, 0.66, 100)]));
    // After Black plays Q16 (a blunder): winrate drops to 45%
    engine.push_evaluation(mock_eval(0.45, -2.0, vec![]));

    let review = AutoReview::run_scan(&mut engine, &records, 19, 50)
        .await
        .unwrap();

    assert_eq!(review.total_moves, 1);
    assert_eq!(review.bad_move_count, 1);
    assert_eq!(review.slack_move_count, 0);

    let m = &review.moves[0];
    assert_eq!(m.quality, MoveQuality::BadMove);
    assert!((m.winrate_before - 0.65).abs() < f64::EPSILON);
    assert!((m.winrate_after - 0.45).abs() < f64::EPSILON);
    assert!((m.winrate_drop - 0.20).abs() < 0.001); // 20% drop
    assert!((m.score_drop - 7.0).abs() < 0.001); // 5.0 - (-2.0) = 7.0
    assert_eq!(m.ai_best_move, Some((3, 3))); // AI recommended D4
    assert_eq!(m.top_suggestions.len(), 1);
}

#[tokio::test]
async fn run_scan_detects_bad_move_by_score_drop() {
    let records = vec![mock_record(1, Color::White, "K10")];
    let mut engine = started_mock_engine().await;

    // Initial eval: White to move. Black winrate = 40% (so White = 60%), Black lead = -4.0
    engine.push_evaluation(mock_eval(0.40, -4.0, vec![mock_suggestion(9, 9, 0.42, 80)]));
    // After White plays K10 (bad): Black winrate = 55%, Black lead = 2.0
    // White's winrate went from 60% to 45% → 15% drop
    // White's score went from +4 to -2 → 6 point loss
    engine.push_evaluation(mock_eval(0.55, 2.0, vec![]));

    let review = AutoReview::run_scan(&mut engine, &records, 19, 50)
        .await
        .unwrap();

    assert_eq!(review.total_moves, 1);
    assert_eq!(review.bad_move_count, 1);

    let m = &review.moves[0];
    assert_eq!(m.quality, MoveQuality::BadMove);
    // White's winrate drop = winrate_after - winrate_before = 0.55 - 0.40 = 0.15
    assert!((m.winrate_drop - 0.15).abs() < 0.001);
    // White's score drop = lead_after - lead_before = 2.0 - (-4.0) = 6.0
    assert!((m.score_drop - 6.0).abs() < 0.001);
    assert_eq!(m.ai_best_move, Some((9, 9))); // AI recommended K10
}

#[tokio::test]
async fn run_scan_classifies_slack_move() {
    let records = vec![
        mock_record(1, Color::Black, "D4"),
        mock_record(2, Color::White, "Q16"),
        mock_record(3, Color::Black, "D16"), // a slight inaccuracy
    ];
    let mut engine = started_mock_engine().await;

    // Initial eval
    engine.push_evaluation(mock_eval(0.50, 0.0, vec![]));
    // After B D4
    engine.push_evaluation(mock_eval(0.52, 1.0, vec![]));
    // After W Q16
    engine.push_evaluation(mock_eval(0.48, -1.0, vec![]));
    // After B D16 (slight mistake: 4% winrate drop, 1.0 score drop)
    engine.push_evaluation(mock_eval(0.44, -2.0, vec![]));

    let review = AutoReview::run_scan(&mut engine, &records, 19, 50)
        .await
        .unwrap();

    assert_eq!(review.total_moves, 3);
    assert_eq!(review.bad_move_count, 0);
    assert_eq!(review.slack_move_count, 1);

    // Move 3 should be slack (4% drop, not quite 8%)
    assert_eq!(review.moves[0].quality, MoveQuality::Good);
    assert_eq!(review.moves[1].quality, MoveQuality::Good);
    let m3 = &review.moves[2];
    assert_eq!(m3.quality, MoveQuality::SlackMove);
    assert!((m3.winrate_drop - 0.04).abs() < 0.001);
    assert!((m3.score_drop - 1.0).abs() < 0.001);
}

#[tokio::test]
async fn run_scan_multi_move_game_with_mixed_qualities() {
    // Simulate a game where Black blunders on move 3 and White throws on move 4
    let records = vec![
        mock_record(1, Color::Black, "D4"),
        mock_record(2, Color::White, "Q16"),
        mock_record(3, Color::Black, "K10"), // blunder
        mock_record(4, Color::White, "R5"),  // blunder
        mock_record(5, Color::Black, "D10"), // recovery
    ];
    let mut engine = started_mock_engine().await;

    // Push evaluations for each position (initial + after each move)
    engine.push_evaluation(mock_eval(0.50, 0.0, vec![])); // initial
    engine.push_evaluation(mock_eval(0.53, 2.0, vec![])); // after B D4
    engine.push_evaluation(mock_eval(0.47, -1.5, vec![])); // after W Q16
    engine.push_evaluation(mock_eval(0.35, -5.0, vec![])); // after B K10 (bad!)
    engine.push_evaluation(mock_eval(0.55, 3.0, vec![])); // after W R5 (bad for W!)
    engine.push_evaluation(mock_eval(0.58, 4.0, vec![])); // after B D10

    let review = AutoReview::run_scan(&mut engine, &records, 19, 50)
        .await
        .unwrap();

    assert_eq!(review.total_moves, 5);
    assert_eq!(review.bad_move_count, 2); // moves 3 and 4
    assert_eq!(review.slack_move_count, 0);

    // Check each move
    // Move 1 (B D4): winrate 0.53-0.50=+0.03 improvement → Good
    assert_eq!(review.moves[0].quality, MoveQuality::Good);

    // Move 2 (W Q16): winrate for W = 0.53-0.47=0.06? Wait...
    // Before: W=50%, Black has 0.50. After W Q16: Black has 0.47.
    // White winrate before = 1-0.50=0.50. After = 1-0.47=0.53.
    // White's winrate_drop = W_after_before - W_before? No...
    // For White: winrate_drop = W_after - W_before in terms of Black's perspective
    // = 0.47 - 0.50 = -0.03 (negative = improvement for White)
    // Actually, let me re-derive. For White move:
    // winrate_drop = winrate_after_black - winrate_before_black
    // = 0.47 - 0.50 = -0.03 → improvement → Good
    assert_eq!(review.moves[1].quality, MoveQuality::Good);

    // Move 3 (B K10): Before Black=0.47, After Black=0.35
    // winrate_drop = 0.47 - 0.35 = 0.12 > 8% → BadMove
    let m3 = &review.moves[2];
    assert_eq!(m3.quality, MoveQuality::BadMove);
    assert!((m3.winrate_drop - 0.12).abs() < 0.01);

    // Move 4 (W R5): Before Black=0.35, After Black=0.55
    // White's winrate_drop = 0.55 - 0.35 = 0.20 > 8% → BadMove
    let m4 = &review.moves[3];
    assert_eq!(m4.quality, MoveQuality::BadMove);
    assert!((m4.winrate_drop - 0.20).abs() < 0.01);

    // Move 5 (B D10): Before Black=0.55, After Black=0.58
    // winrate_drop = 0.55 - 0.58 = -0.03 → improvement → Good
    assert_eq!(review.moves[4].quality, MoveQuality::Good);
}

#[tokio::test]
async fn run_scan_preserves_ai_best_move_and_suggestions() {
    let records = vec![mock_record(1, Color::Black, "Q16")];
    let mut engine = started_mock_engine().await;

    let best = mock_suggestion(3, 15, 0.55, 300); // D4
    let second = mock_suggestion(15, 3, 0.53, 200); // Q4

    engine.push_evaluation(mock_eval(0.50, 0.0, vec![best.clone(), second.clone()]));
    engine.push_evaluation(mock_eval(0.45, -2.0, vec![]));

    let review = AutoReview::run_scan(&mut engine, &records, 19, 50)
        .await
        .unwrap();

    let m = &review.moves[0];
    assert_eq!(m.ai_best_move, Some((best.col, best.row)));
    assert_eq!(m.top_suggestions.len(), 2);
    assert_eq!(m.top_suggestions[0].visits, 300);
    assert_eq!(m.top_suggestions[1].visits, 200);
}

#[test]
fn classify_edge_cases() {
    // Exactly at threshold → BadMove
    assert_eq!(auto_review::classify(0.0800001, 0.0), MoveQuality::BadMove);
    assert_eq!(auto_review::classify(0.0, 1.5000001), MoveQuality::BadMove);

    // Exactly at slack threshold → SlackMove
    assert_eq!(
        auto_review::classify(0.0300001, 0.0),
        MoveQuality::SlackMove
    );
    assert_eq!(
        auto_review::classify(0.0, 0.5000001),
        MoveQuality::SlackMove
    );

    // Just below slack threshold → Good
    assert_eq!(auto_review::classify(0.029, 0.49), MoveQuality::Good);

    // Negative drops (position improved) → Good
    assert_eq!(auto_review::classify(-0.15, -5.0), MoveQuality::Good);
}
