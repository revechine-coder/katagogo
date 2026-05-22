use crate::analysis_parser::{parse_gtp_coordinate, AnalysisParser};
use std::time::Duration;

#[test]
fn parses_modern_info_move_frame_with_pv() {
    let parser = AnalysisParser::new(19);
    let frame = parser
        .parse_line(
            "info move D4 visits 40 edgeVisits 40 utility 0 winrate 0.5523 scoreLead 2.1 order 0 pv D4 Q16 K10 pass",
        )
        .unwrap();

    assert_eq!(frame.candidates.len(), 1);
    let candidate = &frame.candidates[0];
    assert_eq!(candidate.coordinate, (3, 15));
    assert_eq!(candidate.visits, 40);
    assert!((candidate.winrate - 0.5523).abs() < f32::EPSILON);
    assert!((candidate.score_lead - 2.1).abs() < f32::EPSILON);
    assert_eq!(candidate.pv, vec![(3, 15), (15, 3), (9, 9)]);
}

#[test]
fn normalizes_katago_integer_winrate_scale() {
    let parser = AnalysisParser::new(19);
    let frame = parser
        .parse_line("info move Q16 visits 128 winrate 6375 scoreLead -1.5 order 0 pv Q16 D4")
        .unwrap();

    let candidate = &frame.candidates[0];
    assert_eq!(candidate.coordinate, (15, 3));
    assert!((candidate.winrate - 0.6375).abs() < f32::EPSILON);
    assert!((candidate.score_lead + 1.5).abs() < f32::EPSILON);
}

#[test]
fn parses_legacy_search_analyze_candidate_line() {
    let parser = AnalysisParser::new(19);
    let frame = parser
        .parse_line(
            "D4  : T  96.86c W  77.33c S  10.64c ( +5.2 L  +5.2) N      29  --  D4 D5 Q16 pass",
        )
        .unwrap();

    let candidate = &frame.candidates[0];
    assert_eq!(candidate.coordinate, (3, 15));
    assert_eq!(candidate.visits, 29);
    assert!((candidate.winrate - 0.7733).abs() < 0.0001);
    assert!((candidate.score_lead - 10.64).abs() < f32::EPSILON);
    assert_eq!(candidate.pv, vec![(3, 15), (3, 14), (15, 3)]);
}

#[test]
fn parses_multiple_lines_into_one_frame() {
    let parser = AnalysisParser::new(19);
    let lines = vec![
        "noise".to_string(),
        "info move D4 visits 40 winrate 0.55 scoreLead 2.1 order 0 pv D4 Q16".to_string(),
        "info move Q16 visits 12 winrate 0.48 scoreLead -0.7 order 1 pv Q16 D4".to_string(),
    ];

    let frame = parser.parse_lines(&lines).unwrap();

    assert_eq!(frame.candidates.len(), 2);
    assert_eq!(frame.candidates[0].coordinate, (3, 15));
    assert_eq!(frame.candidates[1].coordinate, (15, 3));
}

#[test]
fn throttles_processed_frames() {
    let mut parser = AnalysisParser::with_min_emit_interval(19, Duration::from_secs(60));
    let line = "info move D4 visits 40 winrate 0.55 scoreLead 2.1 order 0 pv D4 Q16";

    assert!(parser.process_line(line).is_some());
    assert!(parser.process_line(line).is_none());
}

#[test]
fn zero_interval_parser_emits_every_valid_line_for_tests_and_mocks() {
    let mut parser = AnalysisParser::with_min_emit_interval(19, Duration::ZERO);
    let line = "info move D4 visits 40 winrate 0.55 scoreLead 2.1 order 0 pv D4 Q16";

    assert!(parser.process_line(line).is_some());
    assert!(parser.process_line(line).is_some());
}

#[test]
fn coordinate_parser_rejects_pass_resign_and_out_of_bounds_vertices() {
    assert_eq!(parse_gtp_coordinate("D4", 19), Some((3, 15)));
    assert_eq!(parse_gtp_coordinate("PASS", 19), None);
    assert_eq!(parse_gtp_coordinate("RESIGN", 19), None);
    assert_eq!(parse_gtp_coordinate("T20", 19), None);
}
