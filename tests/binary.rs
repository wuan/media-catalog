mod common;

use std::process::Command;

use common::Fixture;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_media-catalog")
}

#[test]
fn restricted_path_produces_a_configuration_error_and_nonzero_exit() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let output = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--catalog-root",
            fixture.root().to_str().unwrap(),
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
    assert!(!fixture.public.exists());
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
        "--catalog-root",
        "--media-url-prefix",
        "--thumb-url-prefix",
        "--jobs",
        "--force",
        "--prune",
        "--dry-run",
        "--quiet",
        "--allow-mount-drop",
    ] {
        assert!(stdout.contains(flag), "help missing {flag}: {stdout}");
    }
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
            "--catalog-root",
            fixture.root().to_str().unwrap(),
            "--quiet",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Errors: 1"), "stdout was: {stdout}");
    assert!(stdout.contains("bad.jpg"));
    assert!(fixture.public_path("index.html").exists());
    assert!(fixture.public_path("catalog/2024.json").exists());
    assert!(fixture
        .read_html("2024/05/index.html")
        .contains("class=\"placeholder\""));
}

#[test]
fn progress_is_reported_on_stderr_unless_quiet() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 20, 20, 1);

    let noisy = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--catalog-root",
            fixture.root().to_str().unwrap(),
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
    assert!(stderr.contains("Scanning"), "stderr was: {stderr}");
    assert!(stderr.contains("Scanned 1 items"), "stderr was: {stderr}");
    assert!(stderr.contains("Probing 1 items"), "stderr was: {stderr}");
    assert!(
        stderr.contains("Generating 1 thumbnails"),
        "stderr was: {stderr}"
    );
    assert!(stderr.contains("Rendered"), "stderr was: {stderr}");

    let quiet = Command::new(binary())
        .args([
            "--media-root",
            fixture.media.to_str().unwrap(),
            "--catalog-root",
            fixture.root().to_str().unwrap(),
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
            "--catalog-root",
            fixture.root().to_str().unwrap(),
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
    assert!(fixture.public_path("index.html").exists());
    assert!(fixture.public_path("catalog/2024.json").exists());
    assert!(fixture.thumb_path("2024/05/a.jpg").exists());
}
