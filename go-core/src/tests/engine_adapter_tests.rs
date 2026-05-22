use crate::engine_adapter::{EngineAdapter, EngineConfig, EngineGenMove};
use crate::error::GoCoreError;
use crate::gtp_client::GtpClient;
use crate::render_frame::MoveSuggestion;

#[tokio::test]
async fn mock_engine_adapter_supports_core_engine_flow() {
    let mut engine = crate::engine_adapter::MockEngineAdapter::new();
    engine.push_genmove(EngineGenMove {
        vertex: "Q16".to_string(),
        winrate_black: Some(0.57),
        lead_black: Some(2.5),
        evaluation_accuracy: Some(0.8),
        suggestions: vec![MoveSuggestion {
            col: 10,
            row: 10,
            winrate: 0.57,
            lead: 2.5,
            visits: 128,
            order: 0,
        }],
    });

    engine
        .start(EngineConfig {
            binary_path: "mock".to_string(),
            config_path: "mock.cfg".to_string(),
            model_path: "mock.bin.gz".to_string(),
            board_size: 19,
            timeout_secs: 1.0,
        })
        .await
        .unwrap();
    engine.play("b", "D4").await.unwrap();

    let generated = engine.genmove("w", 19).await.unwrap();

    assert_eq!(generated.vertex, "Q16");
    assert_eq!(generated.suggestions.len(), 1);
    assert!(engine.is_running());
    assert_eq!(
        engine.played_moves(),
        &[("b".to_string(), "D4".to_string())]
    );

    engine.undo().await.unwrap();
    assert!(engine.played_moves().is_empty());

    engine.close().await.unwrap();
    assert!(!engine.is_running());
}

#[tokio::test]
async fn gtp_client_implements_engine_adapter_contract() {
    let mut engine = GtpClient::new();
    let err = EngineAdapter::play(&mut engine, "b", "D4")
        .await
        .expect_err("unstarted engine should reject play");

    assert!(matches!(err, GoCoreError::ProcessNotRunning));
}
