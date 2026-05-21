use crate::game_state::*;
use crate::move_history::MoveHistory;
use crate::render_frame::*;

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
        evaluation_accuracy: None,
    });
    let renderer = BoardRenderer::new();
    let frame = renderer.render(&gs, mh.all_records());
    assert_eq!(frame.stones.len(), 1);
    assert_eq!(frame.move_labels.len(), 1);
    assert_eq!(frame.last_move, Some((3, 15)));
    assert_eq!(frame.move_labels[0].move_number, 1);
}

#[test]
fn test_pass_not_in_labels() {
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
        evaluation_accuracy: None,
    });
    gs.record_move(Color::White, "pass").unwrap();
    mh.push(MoveRecord {
        move_number: 2,
        color: Color::White,
        vertex: "pass".to_string(),
        captured: vec![],
        winrate: None,
        lead: None,
        evaluation_accuracy: None,
    });
    let renderer = BoardRenderer::new();
    let frame = renderer.render(&gs, mh.all_records());
    assert_eq!(frame.move_labels.len(), 1); // only D4, not pass
}

#[test]
fn test_setup_stones_render_without_move_labels() {
    let mut gs = GameState::new(19);
    let mh = MoveHistory::new();
    gs.set_setup_stones(Color::Black, &["D4", "Q16"]).unwrap();

    let renderer = BoardRenderer::new();
    let frame = renderer.render(&gs, mh.all_records());

    assert_eq!(frame.stones.len(), 2);
    assert!(frame.move_labels.is_empty());
    assert!(frame.last_move.is_none());
    assert_eq!(frame.move_count, 0);
    assert_eq!(frame.current_player, "w");
}
