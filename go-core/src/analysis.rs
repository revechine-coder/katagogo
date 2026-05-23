use crate::render_frame::MoveSuggestion;

pub struct AnalysisData {
    pub winrate_black: f64,
    pub lead_black: f64,
    pub evaluation_accuracy: f64,
    pub move_count: u32,
    pub current_player: String,
    pub move_suggestions: Vec<MoveSuggestion>,
}

impl AnalysisData {
    pub fn new() -> Self {
        Self {
            winrate_black: 0.5,
            lead_black: 0.0,
            evaluation_accuracy: 0.0,
            move_count: 0,
            current_player: "b".to_string(),
            move_suggestions: Vec::new(),
        }
    }
}

impl Default for AnalysisData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn winrate_from_trend(raw_winrate: Option<f64>, lead_human: Option<f64>) -> Option<f64> {
    if let Some(wr) = raw_winrate {
        return Some(wr.clamp(0.0, 1.0));
    }

    if let Some(lead) = lead_human {
        return Some((0.5 + 0.5 * (lead / 18.0).tanh()).clamp(0.0, 1.0));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::winrate_from_trend;

    #[test]
    fn trend_winrate_prefers_raw_engine_winrate_when_available() {
        let winrate = winrate_from_trend(Some(0.9), Some(0.0)).unwrap();

        assert!((winrate - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn trend_winrate_estimates_from_score_lead_when_raw_winrate_is_missing() {
        let slight_advantage = winrate_from_trend(None, Some(3.0)).unwrap();
        let large_advantage = winrate_from_trend(None, Some(36.0)).unwrap();
        let slight_disadvantage = winrate_from_trend(None, Some(-3.0)).unwrap();
        let large_disadvantage = winrate_from_trend(None, Some(-36.0)).unwrap();

        assert!(slight_advantage > 0.5);
        assert!(large_advantage > slight_advantage);
        assert!(large_advantage < 1.0);
        assert!(slight_disadvantage < 0.5);
        assert!(large_disadvantage < slight_disadvantage);
        assert!(large_disadvantage > 0.0);
    }

    #[test]
    fn trend_winrate_falls_back_to_raw_winrate_without_lead() {
        let winrate = winrate_from_trend(Some(1.2), None).unwrap();

        assert_eq!(winrate, 1.0);
    }
}
