use crate::game_state::Color;
use crate::play_tree::{AiSnapshot, BoardPoint, GoPlayTree, PlayMove, StoneColor};

#[test]
fn play_extends_the_active_line() {
    let mut tree = GoPlayTree::new();

    let first = tree
        .play(
            PlayMove::stone(Color::Black, BoardPoint::new(3, 3)).with_ai_snapshot(AiSnapshot {
                winrate: 0.53,
                lead: 1.5,
                visits: 128,
            }),
        )
        .expect("first move should be accepted");
    let first_move_number = first.move_number;
    let first_color = first.color;
    let first_visits = first.ai_snapshot.unwrap().visits;
    let second = tree
        .play(PlayMove::stone(Color::White, BoardPoint::new(15, 15)))
        .expect("second move should be accepted");

    assert_eq!(first_move_number, 1);
    assert_eq!(first_color, StoneColor::Black);
    assert_eq!(first_visits, 128);
    assert_eq!(second.move_number, 2);
    assert_eq!(tree.current_node().move_number, 2);
    assert_eq!(tree.current_path(), &[0, 0]);
    assert_eq!(tree.active_line().len(), 2);
}

#[test]
fn undo_moves_to_the_parent_without_deleting_variations() {
    let mut tree = GoPlayTree::new();
    tree.play(PlayMove::stone(Color::Black, BoardPoint::new(3, 3)))
        .unwrap();
    tree.play(PlayMove::stone(Color::White, BoardPoint::new(15, 15)))
        .unwrap();

    let undone = tree.undo().expect("a move should be undone");

    assert_eq!(undone.move_number, 2);
    assert_eq!(tree.current_node().move_number, 1);
    assert_eq!(tree.current_node().children.len(), 1);
    assert_eq!(tree.active_line().len(), 2);
}

#[test]
fn playing_after_undo_creates_a_variation_and_selects_it() {
    let mut tree = GoPlayTree::new();
    tree.play(PlayMove::stone(Color::Black, BoardPoint::new(3, 3)))
        .unwrap();
    tree.play(PlayMove::stone(Color::White, BoardPoint::new(15, 15)))
        .unwrap();
    tree.undo().unwrap();

    let variation = tree
        .play(PlayMove::stone(Color::White, BoardPoint::new(16, 16)))
        .unwrap();

    assert_eq!(variation.move_number, 2);
    tree.undo().unwrap();
    assert_eq!(tree.current_node().children.len(), 2);
    tree.switch_active_branch(1).unwrap();
    assert_eq!(tree.current_path(), &[0, 1]);
    assert_eq!(tree.active_line()[1].point, Some(BoardPoint::new(16, 16)));
}

#[test]
fn switch_active_branch_selects_a_child_from_the_current_node() {
    let mut tree = GoPlayTree::new();
    tree.play(PlayMove::stone(Color::Black, BoardPoint::new(3, 3)))
        .unwrap();
    tree.play(PlayMove::stone(Color::White, BoardPoint::new(15, 15)))
        .unwrap();
    tree.undo().unwrap();
    tree.play(PlayMove::stone(Color::White, BoardPoint::new(16, 16)))
        .unwrap();
    tree.undo().unwrap();

    let selected = tree
        .switch_active_branch(0)
        .expect("first child branch should exist");

    assert_eq!(selected.point, Some(BoardPoint::new(15, 15)));
    assert_eq!(tree.current_path(), &[0, 0]);
    assert_eq!(tree.active_line()[1].point, Some(BoardPoint::new(15, 15)));
}

#[test]
fn pass_move_keeps_empty_color_and_no_board_point() {
    let mut tree = GoPlayTree::new();

    let pass = tree.play(PlayMove::pass(Color::Black)).unwrap();

    assert_eq!(pass.color, StoneColor::Empty);
    assert_eq!(pass.point, None);
    assert_eq!(pass.played_by, Some(Color::Black));
}
