use go_core::gtp_client::GtpClient;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::Duration;

fn make_mock_script(
    temp_dir: &tempfile::TempDir,
    pid_file: &std::path::Path,
) -> std::path::PathBuf {
    let script_path = temp_dir.path().join("mock_gtp.py");
    let content = format!(
        r#"#!/usr/bin/env python3
import os, sys
with open({:?}, 'w') as f:
    f.write(str(os.getpid()))
print('= ', flush=True)
for line in sys.stdin:
    cmd = line.strip()
    if cmd.startswith('quit'):
        print('= ', flush=True)
        break
    print('= ', flush=True)
"#,
        pid_file
    );
    let mut f = std::fs::File::create(&script_path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    script_path
}

fn make_logging_mock_script(
    temp_dir: &tempfile::TempDir,
    pid_file: &std::path::Path,
    command_log: &std::path::Path,
) -> std::path::PathBuf {
    let script_path = temp_dir.path().join("mock_logging_gtp.py");
    let content = format!(
        r#"#!/usr/bin/env python3
import os, sys
with open({:?}, 'w') as f:
    f.write(str(os.getpid()))
print('= ', flush=True)
for line in sys.stdin:
    cmd = line.strip()
    with open({:?}, 'a') as log:
        log.write(cmd + '\n')
    if cmd.startswith('quit'):
        print('= ', flush=True)
        break
    print('= ', flush=True)
"#,
        pid_file, command_log
    );
    let mut f = std::fs::File::create(&script_path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f.flush().unwrap();
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    script_path
}

fn read_pid(path: &std::path::Path) -> Option<u32> {
    for _ in 0..20 {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                return Some(pid);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

fn process_exists(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn watchdog_graceful_exit_kills_child_process() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let pid_file = temp_dir.path().join("mock.pid");
    let script_path = make_mock_script(&temp_dir, &pid_file);
    let fake_config = temp_dir.path().join("fake.cfg");
    let fake_model = temp_dir.path().join("fake.bin.gz");
    std::fs::write(&fake_config, "dummy").unwrap();
    std::fs::write(&fake_model, "dummy").unwrap();

    let mut client = GtpClient::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        client
            .start(
                script_path.to_str().unwrap(),
                fake_config.to_str().unwrap(),
                fake_model.to_str().unwrap(),
                19,
                5.0,
            )
            .await
            .unwrap();
    });

    assert!(client.is_running());

    let pid = read_pid(&pid_file).expect("mock should write its PID");
    assert!(
        process_exists(pid),
        "mock process should be alive before drop"
    );

    drop(client);

    std::thread::sleep(Duration::from_millis(600));
    assert!(
        !process_exists(pid),
        "mock process PID {pid} should be dead after GtpClient drop"
    );
}

#[test]
fn watchdog_self_healing_restarts_and_replays_moves() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let pid_file = temp_dir.path().join("mock.pid");
    let script_path = make_mock_script(&temp_dir, &pid_file);
    let fake_config = temp_dir.path().join("fake.cfg");
    let fake_model = temp_dir.path().join("fake.bin.gz");
    std::fs::write(&fake_config, "dummy").unwrap();
    std::fs::write(&fake_model, "dummy").unwrap();

    let mut client = GtpClient::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        client
            .start(
                script_path.to_str().unwrap(),
                fake_config.to_str().unwrap(),
                fake_model.to_str().unwrap(),
                19,
                5.0,
            )
            .await
            .unwrap();

        client.play("b", "D4").await.unwrap();
        client.play("w", "Q16").await.unwrap();
        client.play("b", "D16").await.unwrap();
    });

    assert!(client.is_running());

    // Read the PID before killing
    let pid_before = read_pid(&pid_file).expect("mock should have written PID");

    // Force-kill the child process via SIGKILL
    let kill_status = Command::new("kill")
        .arg("-9")
        .arg(pid_before.to_string())
        .status()
        .unwrap();
    assert!(kill_status.success(), "kill -9 should succeed");

    // Verify GtpClient detects the dead process (try_wait reaps the zombie)
    std::thread::sleep(Duration::from_millis(200));
    assert!(
        !client.is_running(),
        "GtpClient should report not running after child was killed"
    );

    // Delete PID file so we can detect the new process writing a fresh one
    let _ = std::fs::remove_file(&pid_file);

    // Trigger self-healing by playing a move
    rt.block_on(async {
        client.play("w", "R5").await.unwrap();
    });

    assert!(
        client.is_running(),
        "client should be running after self-healing"
    );

    let pid_after = read_pid(&pid_file).expect("restarted mock should write its PID");
    assert!(
        process_exists(pid_after),
        "restarted process PID {pid_after} should be alive"
    );
    assert_ne!(
        pid_before, pid_after,
        "restarted process should have a different PID (old={pid_before}, new={pid_after})"
    );

    // Verify we can continue playing after heal
    rt.block_on(async {
        client.play("b", "K10").await.unwrap();
    });
    assert!(client.is_running());

    drop(client);
}

#[test]
fn watchdog_self_healing_does_not_replay_moves_after_clear_board() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let pid_file = temp_dir.path().join("mock.pid");
    let command_log = temp_dir.path().join("commands.log");
    let script_path = make_logging_mock_script(&temp_dir, &pid_file, &command_log);
    let fake_config = temp_dir.path().join("fake.cfg");
    let fake_model = temp_dir.path().join("fake.bin.gz");
    std::fs::write(&fake_config, "dummy").unwrap();
    std::fs::write(&fake_model, "dummy").unwrap();

    let mut client = GtpClient::new();
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        client
            .start(
                script_path.to_str().unwrap(),
                fake_config.to_str().unwrap(),
                fake_model.to_str().unwrap(),
                19,
                5.0,
            )
            .await
            .unwrap();

        client.play("b", "D4").await.unwrap();
        client.play("w", "Q16").await.unwrap();
        client.clear_board(Some(5.0)).await.unwrap();
    });

    let pid_before = read_pid(&pid_file).expect("mock should have written PID");
    let kill_status = Command::new("kill")
        .arg("-9")
        .arg(pid_before.to_string())
        .status()
        .unwrap();
    assert!(kill_status.success(), "kill -9 should succeed");
    std::thread::sleep(Duration::from_millis(200));
    assert!(!client.is_running());

    std::fs::write(&command_log, "").unwrap();
    let _ = std::fs::remove_file(&pid_file);

    rt.block_on(async {
        client.play("b", "K10").await.unwrap();
    });

    let commands = std::fs::read_to_string(&command_log).unwrap();
    assert!(
        !commands.contains("play b D4") && !commands.contains("play w Q16"),
        "old moves should not be replayed after clear_board; commands were:\n{commands}"
    );
    assert!(
        commands.contains("play b K10"),
        "new move should still be sent after healing; commands were:\n{commands}"
    );

    drop(client);
}
