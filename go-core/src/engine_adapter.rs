use crate::error::{GoCoreError, Result};
use crate::render_frame::MoveSuggestion;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug, Clone, PartialEq)]
pub struct PositionEval {
    pub winrate_black: f64,
    pub lead_black: f64,
    pub move_suggestions: Vec<MoveSuggestion>,
}

pub type EngineFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

#[derive(Debug, Clone, PartialEq)]
pub struct EngineConfig {
    pub binary_path: String,
    pub config_path: String,
    pub model_path: String,
    pub board_size: u8,
    pub timeout_secs: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineGenMove {
    pub vertex: String,
    pub winrate_black: Option<f64>,
    pub lead_black: Option<f64>,
    pub evaluation_accuracy: Option<f64>,
    pub suggestions: Vec<MoveSuggestion>,
}

pub trait EngineAdapter {
    fn start<'a>(&'a mut self, config: EngineConfig) -> EngineFuture<'a, ()>;
    fn close<'a>(&'a mut self) -> EngineFuture<'a, ()>;
    fn play<'a>(&'a mut self, color: &'a str, vertex: &'a str) -> EngineFuture<'a, ()>;
    fn genmove<'a>(&'a mut self, color: &'a str, board_size: u8)
        -> EngineFuture<'a, EngineGenMove>;
    fn undo<'a>(&'a mut self) -> EngineFuture<'a, ()>;
    fn final_score<'a>(&'a mut self) -> EngineFuture<'a, String>;
    fn quick_analyze<'a>(
        &'a mut self,
        color: &'a str,
        board_size: u8,
    ) -> EngineFuture<'a, Vec<MoveSuggestion>>;
    fn analyze_ownership<'a>(&'a mut self, board_size: u8) -> EngineFuture<'a, Vec<f64>>;
    fn evaluate_position<'a>(
        &'a mut self,
        color: &'a str,
        board_size: u8,
        num_visits: i32,
    ) -> EngineFuture<'a, PositionEval>;
    fn command<'a>(
        &'a mut self,
        command: &'a str,
        timeout: Option<f64>,
    ) -> EngineFuture<'a, String>;
    fn clear_board<'a>(&'a mut self, timeout: Option<f64>) -> EngineFuture<'a, ()>;
    fn is_running(&mut self) -> bool;
}

#[derive(Debug, Default)]
pub struct MockEngineAdapter {
    running: bool,
    played_moves: Vec<(String, String)>,
    genmoves: VecDeque<EngineGenMove>,
    suggestions: Vec<MoveSuggestion>,
    ownership: Vec<f64>,
    evaluations: VecDeque<PositionEval>,
}

impl MockEngineAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_genmove(&mut self, genmove: EngineGenMove) {
        self.genmoves.push_back(genmove);
    }

    pub fn push_evaluation(&mut self, eval: PositionEval) {
        self.evaluations.push_back(eval);
    }

    pub fn played_moves(&self) -> &[(String, String)] {
        &self.played_moves
    }
}

impl EngineAdapter for MockEngineAdapter {
    fn start<'a>(&'a mut self, _config: EngineConfig) -> EngineFuture<'a, ()> {
        Box::pin(async move {
            self.running = true;
            Ok(())
        })
    }

    fn close<'a>(&'a mut self) -> EngineFuture<'a, ()> {
        Box::pin(async move {
            self.running = false;
            Ok(())
        })
    }

    fn play<'a>(&'a mut self, color: &'a str, vertex: &'a str) -> EngineFuture<'a, ()> {
        Box::pin(async move {
            if !self.running {
                return Err(GoCoreError::ProcessNotRunning);
            }
            self.played_moves
                .push((color.to_string(), vertex.to_string()));
            Ok(())
        })
    }

    fn genmove<'a>(
        &'a mut self,
        _color: &'a str,
        _board_size: u8,
    ) -> EngineFuture<'a, EngineGenMove> {
        Box::pin(async move {
            if !self.running {
                return Err(GoCoreError::ProcessNotRunning);
            }
            self.genmoves
                .pop_front()
                .ok_or_else(|| GoCoreError::Rejected("mock genmove queue is empty".to_string()))
        })
    }

    fn undo<'a>(&'a mut self) -> EngineFuture<'a, ()> {
        Box::pin(async move {
            self.played_moves.pop();
            Ok(())
        })
    }

    fn final_score<'a>(&'a mut self) -> EngineFuture<'a, String> {
        Box::pin(async move { Ok("0".to_string()) })
    }

    fn quick_analyze<'a>(
        &'a mut self,
        _color: &'a str,
        _board_size: u8,
    ) -> EngineFuture<'a, Vec<MoveSuggestion>> {
        Box::pin(async move { Ok(self.suggestions.clone()) })
    }

    fn analyze_ownership<'a>(&'a mut self, _board_size: u8) -> EngineFuture<'a, Vec<f64>> {
        Box::pin(async move { Ok(self.ownership.clone()) })
    }

    fn evaluate_position<'a>(
        &'a mut self,
        _color: &'a str,
        _board_size: u8,
        _num_visits: i32,
    ) -> EngineFuture<'a, PositionEval> {
        Box::pin(async move {
            self.evaluations
                .pop_front()
                .ok_or_else(|| GoCoreError::Rejected("mock evaluations exhausted".to_string()))
        })
    }

    fn command<'a>(
        &'a mut self,
        command: &'a str,
        _timeout: Option<f64>,
    ) -> EngineFuture<'a, String> {
        Box::pin(async move {
            match command {
                "clear_board" => {
                    self.played_moves.clear();
                    Ok(String::new())
                }
                _ => Ok(String::new()),
            }
        })
    }

    fn clear_board<'a>(&'a mut self, _timeout: Option<f64>) -> EngineFuture<'a, ()> {
        Box::pin(async move {
            self.played_moves.clear();
            Ok(())
        })
    }

    fn is_running(&mut self) -> bool {
        self.running
    }
}
