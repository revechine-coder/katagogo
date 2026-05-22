use crate::error::{GoCoreError, Result};
use crate::game_state::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoneColor {
    Empty,
    Black,
    White,
}

impl From<Color> for StoneColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Black => StoneColor::Black,
            Color::White => StoneColor::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardPoint {
    pub x: u8,
    pub y: u8,
}

impl BoardPoint {
    pub fn new(x: u8, y: u8) -> Self {
        BoardPoint { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiSnapshot {
    pub winrate: f32,
    pub lead: f32,
    pub visits: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayKind {
    Stone(BoardPoint),
    Pass,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayMove {
    pub played_by: Color,
    pub kind: PlayKind,
    pub ai_snapshot: Option<AiSnapshot>,
}

impl PlayMove {
    pub fn stone(played_by: Color, point: BoardPoint) -> Self {
        PlayMove {
            played_by,
            kind: PlayKind::Stone(point),
            ai_snapshot: None,
        }
    }

    pub fn pass(played_by: Color) -> Self {
        PlayMove {
            played_by,
            kind: PlayKind::Pass,
            ai_snapshot: None,
        }
    }

    pub fn with_ai_snapshot(mut self, ai_snapshot: AiSnapshot) -> Self {
        self.ai_snapshot = Some(ai_snapshot);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayNode {
    pub point: Option<BoardPoint>,
    pub color: StoneColor,
    pub played_by: Option<Color>,
    pub move_number: u32,
    pub ai_snapshot: Option<AiSnapshot>,
    pub children: Vec<PlayNode>,
    pub active_child: Option<usize>,
}

impl PlayNode {
    fn root() -> Self {
        PlayNode {
            point: None,
            color: StoneColor::Empty,
            played_by: None,
            move_number: 0,
            ai_snapshot: None,
            children: Vec::new(),
            active_child: None,
        }
    }

    fn from_move(play_move: PlayMove, move_number: u32) -> Self {
        let (point, color) = match play_move.kind {
            PlayKind::Stone(point) => (Some(point), StoneColor::from(play_move.played_by)),
            PlayKind::Pass => (None, StoneColor::Empty),
        };

        PlayNode {
            point,
            color,
            played_by: Some(play_move.played_by),
            move_number,
            ai_snapshot: play_move.ai_snapshot,
            children: Vec::new(),
            active_child: None,
        }
    }

    fn matches_move(&self, play_move: PlayMove) -> bool {
        self.point
            == match play_move.kind {
                PlayKind::Stone(point) => Some(point),
                PlayKind::Pass => None,
            }
            && self.played_by == Some(play_move.played_by)
            && self.color
                == match play_move.kind {
                    PlayKind::Stone(_) => StoneColor::from(play_move.played_by),
                    PlayKind::Pass => StoneColor::Empty,
                }
    }
}

#[derive(Debug, Clone)]
pub struct GoPlayTree {
    root: PlayNode,
    current_path: Vec<usize>,
}

impl GoPlayTree {
    pub fn new() -> Self {
        GoPlayTree {
            root: PlayNode::root(),
            current_path: Vec::new(),
        }
    }

    pub fn root(&self) -> &PlayNode {
        &self.root
    }

    pub fn current_path(&self) -> &[usize] {
        &self.current_path
    }

    pub fn current_node(&self) -> &PlayNode {
        self.node_at_path(&self.current_path)
            .expect("current_path should always point to a valid node")
    }

    pub fn play(&mut self, play_move: PlayMove) -> Result<&PlayNode> {
        let move_number = self.current_node().move_number + 1;
        let child_index = {
            let current = self.current_node_mut();
            if let Some(index) = current
                .children
                .iter()
                .position(|child| child.matches_move(play_move))
            {
                current.active_child = Some(index);
                index
            } else {
                current
                    .children
                    .push(PlayNode::from_move(play_move, move_number));
                let index = current.children.len() - 1;
                current.active_child = Some(index);
                index
            }
        };

        self.current_path.push(child_index);
        Ok(self.current_node())
    }

    pub fn switch_active_branch(&mut self, child_index: usize) -> Result<&PlayNode> {
        let current = self.current_node_mut();
        if child_index >= current.children.len() {
            return Err(GoCoreError::InvalidArgument(format!(
                "branch index {child_index} out of range; node has {} child branches",
                current.children.len()
            )));
        }

        current.active_child = Some(child_index);
        self.current_path.push(child_index);
        Ok(self.current_node())
    }

    pub fn undo(&mut self) -> Option<PlayNode> {
        if self.current_path.is_empty() {
            return None;
        }

        let undone = self.current_node().clone();
        self.current_path.pop();
        Some(undone)
    }

    pub fn active_line(&self) -> Vec<&PlayNode> {
        let mut line = Vec::new();
        let mut node = &self.root;

        while let Some(child_index) = node.active_child {
            let Some(child) = node.children.get(child_index) else {
                break;
            };
            line.push(child);
            node = child;
        }

        line
    }

    fn current_node_mut(&mut self) -> &mut PlayNode {
        Self::node_at_path_mut(&mut self.root, &self.current_path)
            .expect("current_path should always point to a valid node")
    }

    fn node_at_path(&self, path: &[usize]) -> Option<&PlayNode> {
        let mut node = &self.root;
        for &index in path {
            node = node.children.get(index)?;
        }
        Some(node)
    }

    fn node_at_path_mut<'a>(node: &'a mut PlayNode, path: &[usize]) -> Option<&'a mut PlayNode> {
        if let Some((&index, rest)) = path.split_first() {
            let child = node.children.get_mut(index)?;
            Self::node_at_path_mut(child, rest)
        } else {
            Some(node)
        }
    }
}

impl Default for GoPlayTree {
    fn default() -> Self {
        Self::new()
    }
}
