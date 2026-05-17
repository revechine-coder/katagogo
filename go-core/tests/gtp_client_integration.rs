use go_core::gtp_client::GtpClient;
use std::path::PathBuf;

#[tokio::test]
async fn gtp_client_can_play_and_generate_move() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("go-core should live inside project root")
        .to_path_buf();
    let engine_dir = project_root.join("kata-engine");
    let binary_path = engine_dir.join("katago-eigen");
    let config_path = engine_dir.join("gtp.cfg");
    let model_path = engine_dir.join("lionffen_b24c64.bin.gz");

    let mut client = GtpClient::new();
    client
        .start(
            binary_path.to_str().expect("binary path should be utf8"),
            config_path.to_str().expect("config path should be utf8"),
            model_path.to_str().expect("model path should be utf8"),
            19,
            30.0,
        )
        .await
        .expect("KataGo should start");

    client.play("b", "D4").await.expect("human move should be accepted");
    let (vertex, _, _) = client
        .genmove("w")
        .await
        .expect("KataGo should generate a reply");

    assert!(!vertex.is_empty());
    client.close().await.expect("KataGo should close cleanly");
}
