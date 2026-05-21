use crate::game_state::*;

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

#[test]
fn test_capture_single_stone_after_last_liberty_removed() {
    let mut gs = GameState::new(9);
    gs.record_move(Color::White, "E5").unwrap();
    gs.record_move(Color::Black, "E6").unwrap();
    gs.record_move(Color::Black, "D5").unwrap();
    gs.record_move(Color::Black, "F5").unwrap();

    let rec = gs.record_move(Color::Black, "E4").unwrap();

    assert_eq!(gs.stone_at(4, 4), None);
    assert_eq!(rec.captured, vec!["E5"]);
    assert_eq!(gs.captures(), (1, 0));
}

#[test]
fn test_suicide_move_is_rejected_when_it_captures_nothing() {
    let mut gs = GameState::new(9);
    gs.record_move(Color::Black, "E6").unwrap();
    gs.record_move(Color::Black, "D5").unwrap();
    gs.record_move(Color::Black, "F5").unwrap();
    gs.record_move(Color::Black, "E4").unwrap();

    let result = gs.record_move(Color::White, "E5");

    assert!(result.is_err());
    assert_eq!(gs.stone_at(4, 4), None);
    assert_eq!(gs.move_count(), 4);
}

#[test]
fn test_setup_stones_do_not_count_as_moves_and_white_plays_next() {
    let mut gs = GameState::new(19);

    gs.set_setup_stones(Color::Black, &["D4", "Q16", "D16"])
        .unwrap();

    assert_eq!(gs.move_count(), 0);
    assert_eq!(gs.current_player(), Color::White);
    assert_eq!(gs.stone_at(3, 15), Some(Color::Black));
    assert_eq!(gs.stone_at(15, 3), Some(Color::Black));
    assert_eq!(gs.stone_at(3, 3), Some(Color::Black));
}
