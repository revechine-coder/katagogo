use go_core::gtp_client::GtpClient;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn gtp_client_command_timeout_returns_when_engine_stops_replying() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let engine_path = temp_dir.path().join("fake-gtp-engine.sh");
    fs::write(
        &engine_path,
        r#"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    "genmove "*)
      sleep 30
      ;;
    *)
      printf '= \n\n'
      ;;
  esac
done
"#,
    )
    .expect("fake engine should be written");
    let mut perms = fs::metadata(&engine_path)
        .expect("fake engine metadata should be readable")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&engine_path, perms).expect("fake engine should be executable");

    let (tx, rx) = mpsc::channel();
    let engine_path_for_thread = engine_path.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("runtime should be created");
        let result = rt.block_on(async {
            let mut client = GtpClient::new();
            client
                .start(
                    engine_path_for_thread
                        .to_str()
                        .expect("fake engine path should be utf8"),
                    "unused.cfg",
                    "unused-model.bin.gz",
                    19,
                    1.0,
                )
                .await?;
            client.command("genmove w", Some(0.2)).await
        });
        let _ = tx.send(result);
    });

    let result = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("command timeout should return instead of hanging");

    assert!(
        result
            .expect_err("unanswered command should fail")
            .to_string()
            .contains("timed out"),
        "unanswered command should return a timeout error"
    );
}

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

    client
        .play("b", "D4")
        .await
        .expect("human move should be accepted");
    let (vertex, _, _, _, _) = client
        .genmove("w", 19)
        .await
        .expect("KataGo should generate a reply");

    assert!(!vertex.is_empty());
    client.close().await.expect("KataGo should close cleanly");
}

#[tokio::test]
async fn gtp_client_genmove_returns_analysis_data() {
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

    client
        .play("b", "D4")
        .await
        .expect("human move should be accepted");
    client
        .play("w", "Q16")
        .await
        .expect("white move should be accepted");

    let (_, winrate, lead, accuracy, _suggestions) = client
        .genmove("b", 19)
        .await
        .expect("KataGo should generate move with analysis");

    assert!(winrate.is_some());
    assert!(lead.is_some());
    assert!(accuracy.unwrap_or(0.0) > 0.0);

    client.close().await.expect("KataGo should close cleanly");
}

#[tokio::test]
async fn gtp_client_reports_running_after_start() {
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

    assert!(client.is_running());

    client.close().await.expect("KataGo should close cleanly");
}

#[tokio::test]
async fn gtp_client_undo_keeps_engine_board_in_sync() {
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

    client
        .play("b", "D4")
        .await
        .expect("first move should be accepted");
    client
        .play("w", "Q16")
        .await
        .expect("second move should be accepted");

    client.undo().await.expect("undo should be accepted");
    client
        .play("w", "Q16")
        .await
        .expect("same move should be legal after undo");

    client.close().await.expect("KataGo should close cleanly");
}

#[tokio::test]
async fn gtp_client_undo_after_genmove_keeps_engine_board_in_sync() {
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

    client
        .play("b", "D4")
        .await
        .expect("human move should be accepted");
    let (vertex, _, _, _, _) = client
        .genmove("w", 19)
        .await
        .expect("KataGo should generate a reply");

    client.undo().await.expect("undo should be accepted");
    client
        .play("w", &vertex)
        .await
        .expect("generated move point should be legal after undo");

    client.close().await.expect("KataGo should close cleanly");
}

#[tokio::test]
async fn gtp_client_can_continue_after_genmove_quick_analyze_cycle() {
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

    client
        .play("b", "D4")
        .await
        .expect("human move should be accepted");
    let (ai_vertex, _, _, _, _) = client
        .genmove("w", 19)
        .await
        .expect("KataGo should generate a reply");
    let suggestions = client
        .quick_analyze("b", 19)
        .await
        .expect("post-move suggestions should be readable");
    assert!(
        !suggestions.is_empty(),
        "post-move analysis should include candidate suggestions"
    );

    let next_human_vertex = if ai_vertex.eq_ignore_ascii_case("K10") {
        "T19"
    } else {
        "K10"
    };

    client
        .play("b", next_human_vertex)
        .await
        .expect("next human move should still be accepted");
    let (vertex, _, _, _, _) = client
        .genmove("w", 19)
        .await
        .expect("KataGo should generate a second reply");

    assert!(!vertex.is_empty());
    client.close().await.expect("KataGo should close cleanly");
}

#[tokio::test]
async fn default_model_quick_analyze_returns_without_hanging() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("go-core should live inside project root")
        .to_path_buf();
    let engine_dir = project_root.join("kata-engine");
    let binary_path = engine_dir.join("katago-eigen");
    let config_path = engine_dir.join("gtp.cfg");
    let model_path = engine_dir.join("lionffen_b6c64_3x3_v10.txt.gz");

    let result = tokio::time::timeout(Duration::from_secs(8), async {
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

        client
            .play("b", "D4")
            .await
            .expect("human move should be accepted");
        client
            .genmove("w", 19)
            .await
            .expect("KataGo should generate a reply");
        client
            .quick_analyze("b", 19)
            .await
            .expect("post-move suggestions should return");

        client.close().await.expect("KataGo should close cleanly");
    })
    .await;

    assert!(result.is_ok(), "quick analysis hung with the default model");
}

#[tokio::test]
async fn quick_analyze_produces_candidates_at_low_playing_visits() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("go-core should live inside project root")
        .to_path_buf();
    let engine_dir = project_root.join("kata-engine");
    let binary_path = engine_dir.join("katago-eigen");
    let config_path = engine_dir.join("gtp.cfg");
    let model_path = engine_dir.join("lionffen_b6c64_3x3_v10.txt.gz");

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
    client
        .set_param("maxVisits", "1")
        .await
        .expect("low playing visits should be accepted");

    client
        .play("b", "D4")
        .await
        .expect("human move should be accepted");
    client
        .genmove("w", 19)
        .await
        .expect("KataGo should generate a reply");

    let suggestions = client
        .quick_analyze("b", 19)
        .await
        .expect("suggestions should be readable");

    assert!(
        !suggestions.is_empty(),
        "quick analysis should be deeper than low playing visits"
    );

    client.close().await.expect("KataGo should close cleanly");
}
