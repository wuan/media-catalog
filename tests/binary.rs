mod common;

use std::process::Command;

use common::Fixture;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_media-catalog")
}

fn server_binary() -> &'static str {
    env!("CARGO_BIN_EXE_media-catalog-server")
}

#[test]
fn restricted_path_produces_a_configuration_error_and_nonzero_exit() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let output = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--quiet",
        ])
        .env("PATH", "/nonexistent-bin")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ffmpeg is required"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("PATH"));
    assert!(!fixture.data.exists());
}

#[test]
fn missing_required_roots_are_rejected() {
    let output = Command::new(binary()).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--media-root"));
}

#[test]
fn help_starts_and_documents_flags() {
    let output = Command::new(binary()).arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--media-root",
        "--data-root",
        "--state-root",
        "--media-url-prefix",
        "--thumb-url-prefix",
        "--jobs",
        "--force",
        "--prune",
        "--dry-run",
        "--quiet",
        "--allow-mount-drop",
        "--events",
        "--include-location",
        "--location-preview",
    ] {
        assert!(stdout.contains(flag), "help missing {flag}: {stdout}");
    }
    assert!(
        !stdout.contains("--catalog-root"),
        "help still documents --catalog-root: {stdout}"
    );
    assert!(
        stdout.contains("authored events content file"),
        "help does not describe --events: {stdout}"
    );
    assert!(
        !stdout.contains("./events.toml"),
        "help still documents the implicit events default: {stdout}"
    );
}

#[test]
fn server_help_documents_shared_configuration_flags() {
    let output = Command::new(server_binary())
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--config",
        "--bind",
        "--max-streams",
        "--data-root",
        "--media-root",
    ] {
        assert!(stdout.contains(flag), "help missing {flag}: {stdout}");
    }
}

#[test]
fn server_missing_roots_name_flags_and_keys() {
    let fixture = Fixture::new();
    let output = Command::new(server_binary())
        .current_dir(fixture.root())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--data-root"), "stderr was: {stderr}");
    assert!(stderr.contains("data_root"), "stderr was: {stderr}");

    let output = Command::new(server_binary())
        .args(["--data-root", fixture.data.to_str().unwrap()])
        .current_dir(fixture.root())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--media-root"), "stderr was: {stderr}");
    assert!(stderr.contains("media_root"), "stderr was: {stderr}");
}

#[test]
fn server_missing_config_file_exits_nonzero_naming_it() {
    let fixture = Fixture::new();
    let missing = fixture.root().join("missing.toml");
    let output = Command::new(server_binary())
        .args(["--config", missing.to_str().unwrap()])
        .current_dir(fixture.root())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("configuration file not found"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("missing.toml"), "stderr was: {stderr}");
}

#[test]
fn missing_events_flag_path_is_rejected_before_building() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let output = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--events",
            fixture.root().join("missing.toml").to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("events file not found"),
        "stderr was: {stderr}"
    );
    assert!(!fixture.data.exists());
    assert!(!fixture.state_db().exists());
}

#[test]
fn location_preview_without_location_is_rejected_before_building() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let output = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--location-preview",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("location_preview"), "stderr was: {stderr}");
    assert!(stderr.contains("include_location"), "stderr was: {stderr}");
    assert!(!fixture.data.exists());
    assert!(!fixture.state_db().exists());
}

#[test]
fn failing_item_keeps_catalog_and_exits_nonzero() {
    let fixture = Fixture::new();
    fixture.write_bytes("2024/05/bad.jpg", b"this is not a jpeg");
    fixture.write_image("2024/05/good.jpg", 20, 20, 1);

    let output = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Errors: 1"), "stdout was: {stdout}");
    assert!(stdout.contains("bad.jpg"));
    assert!(fixture.data_path("catalog/index.json").exists());
    assert!(fixture.data_path("catalog/2024.json").exists());
    assert!(fixture
        .read_json("catalog/2024.json")
        .contains("\"thumb_available\": false"));
    assert!(!fixture.data_path("2024/05/index.html").exists());
}

#[test]
fn progress_is_reported_on_stderr_unless_quiet() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let noisy = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--jobs",
            "2",
        ])
        .output()
        .unwrap();

    assert!(
        noisy.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&noisy.stderr)
    );
    let stderr = String::from_utf8_lossy(&noisy.stderr);
    assert!(stderr.contains("Data root:"), "stderr was: {stderr}");
    assert!(stderr.contains("State root:"), "stderr was: {stderr}");
    assert!(
        !stderr.to_ascii_lowercase().contains("catalog root"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("Scanning"), "stderr was: {stderr}");
    assert!(stderr.contains("Scanned 1 items"), "stderr was: {stderr}");
    assert!(stderr.contains("Probing 1 items"), "stderr was: {stderr}");
    assert!(
        stderr.contains("Generating 1 thumbnails"),
        "stderr was: {stderr}"
    );
    assert!(
        stderr.contains("Publishing JSON catalog"),
        "stderr was: {stderr}"
    );

    let quiet = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--jobs",
            "2",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        quiet.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&quiet.stderr)
    );
    let stderr = String::from_utf8_lossy(&quiet.stderr);
    assert!(stderr.is_empty(), "quiet run wrote to stderr: {stderr}");
}

#[test]
fn documented_command_builds_a_fixture() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let output = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--data-root",
            fixture.data.to_str().unwrap(),
            "--state-root",
            fixture.state.to_str().unwrap(),
            "--jobs",
            "2",
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Scanned: 1 items"));
    assert!(stdout.contains("Thumbnails: 1 generated"));
    assert!(fixture.data_path("catalog/index.json").exists());
    assert!(fixture.data_path("catalog/2024.json").exists());
    assert!(fixture.thumb_path("2024/05/a.jpg").exists());
}
