pub struct AnalysisData {
    pub winrate_black: f64,
    pub lead: f64,
    pub move_count: u32,
    pub current_player: String,
}

impl AnalysisData {
    pub fn new() -> Self {
        AnalysisData {
            winrate_black: 0.5,
            lead: 0.0,
            move_count: 0,
            current_player: "b".to_string(),
        }
    }
}
