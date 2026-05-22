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

#[test]
fn c_render_frame_view_points_at_rust_owned_buffers() {
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
    let buffer = RenderFrameBuffer::from_frame(frame, Vec::new());
    let view = buffer.view();

    assert_eq!(view.board_size, 19);
    assert_eq!(view.current_player, 1);
    assert_eq!(view.stones_len, 1);
    assert_eq!(view.move_labels_len, 1);
    assert_eq!(view.star_points_len, 9);
    assert_eq!(view.last_move_col, 3);
    assert_eq!(view.last_move_row, 15);

    unsafe {
        assert_eq!((*view.stones).col, 3);
        assert_eq!((*view.stones).row, 15);
        assert_eq!((*view.move_labels).move_number, 1);
        assert_eq!((*view.star_points).col, 3);
    }
}

#[test]
fn c_render_frame_view_exposes_move_suggestions_without_copying() {
    let frame = RenderFrame {
        board_size: 19,
        stones: Vec::new(),
        last_move: None,
        move_labels: Vec::new(),
        star_points: vec![(3, 3)],
        move_count: 0,
        current_player: "b".to_string(),
        captures_black: 0,
        captures_white: 0,
    };
    let suggestions = vec![MoveSuggestion {
        col: 10,
        row: 10,
        winrate: 0.57,
        lead: 2.25,
        visits: 512,
        order: 1,
    }];

    let buffer = RenderFrameBuffer::from_frame(frame, suggestions);
    let view = buffer.view();

    assert_eq!(view.suggestions_len, 1);
    unsafe {
        assert_eq!((*view.suggestions).col, 10);
        assert_eq!((*view.suggestions).visits, 512);
    }
}
