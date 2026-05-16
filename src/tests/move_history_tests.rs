use crate::game_state::{Color, MoveRecord};
use crate::move_history::MoveHistory;

fn dummy_record(move_number: u32) -> MoveRecord {
    MoveRecord {
        move_number,
        color: if move_number % 2 == 1 {
            Color::Black
        } else {
            Color::White
        },
        vertex: if move_number % 2 == 1 {
            "D4".to_string()
        } else {
            "Q16".to_string()
        },
        captured: vec![],
        winrate: None,
        lead: None,
    }
}

#[test]
fn test_push_and_undo() {
    let mut hist = MoveHistory::new();
    assert!(hist.undo().is_none());

    hist.push(dummy_record(1));
    hist.push(dummy_record(2));

    let undone = hist.undo().unwrap();
    assert_eq!(undone.move_number, 2);
    assert_eq!(hist.current_index(), 1);

    let redo = hist.redo().unwrap();
    assert_eq!(redo.move_number, 2);
    assert_eq!(hist.current_index(), 2);
}

#[test]
fn test_jump_to_start() {
    let mut hist = MoveHistory::new();
    for i in 1..=5 {
        hist.push(dummy_record(i));
    }
    let undone = hist.jump_to(0);
    assert_eq!(undone.len(), 5);
    assert_eq!(hist.current_index(), 0);
    assert!(hist.undo().is_none());
}

#[test]
fn test_jump_to_middle() {
    let mut hist = MoveHistory::new();
    for i in 1..=5 {
        hist.push(dummy_record(i));
    }
    let undone = hist.jump_to(3);
    assert_eq!(undone.len(), 2);
    assert_eq!(hist.current_index(), 3);
    assert_eq!(hist.last().unwrap().move_number, 3);
}

#[test]
fn test_truncate_on_push_after_undo() {
    let mut hist = MoveHistory::new();
    for i in 1..=3 {
        hist.push(dummy_record(i));
    }
    hist.undo();
    hist.push(dummy_record(4));
    assert_eq!(hist.len(), 3);
    assert_eq!(hist.last().unwrap().move_number, 4);
}

#[test]
fn test_empty_history() {
    let hist = MoveHistory::new();
    assert!(hist.last().is_none());
    assert!(hist.is_empty());
}
