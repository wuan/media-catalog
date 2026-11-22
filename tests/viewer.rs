use std::process::Command;

use serde_json::Value;

#[test]
fn viewer_helpers_cover_url_album_zoom_and_swipe_rules() {
    let node_available = Command::new("node")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !node_available {
        eprintln!("skipping viewer JS test: node is unavailable");
        return;
    }

    let script = format!("{}/tests/js/viewer_check.mjs", env!("CARGO_MANIFEST_DIR"));
    let asset = format!("{}/assets/viewer.js", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("node")
        .args([script.as_str(), asset.as_str()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "node failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["item"], "2024/05/a.jpg");
    assert_eq!(result["noItem"], Value::Null);
    assert_eq!(
        result["encodedItem"],
        "/search.html?q=alpha&kind=video&item=2024%2F05%2Fa.jpg"
    );
    assert_eq!(result["preservedItem"], "/search.html?q=alpha&kind=video");
    assert_eq!(result["fragment"], "/events/trip/#lisbon");
    assert_eq!(result["found"], 1);
    assert_eq!(result["missing"], -1);
    assert_eq!(result["next"], 1);
    assert_eq!(result["previous"], 0);
    assert_eq!(result["beforeFirst"], -1);
    assert_eq!(result["afterLast"], -1);
    assert_eq!(result["zoomLow"], 1.0);
    assert_eq!(result["zoomHigh"], 4.0);
    assert_eq!(result["panFit"]["x"], 0.0);
    assert_eq!(result["panFit"]["y"], 0.0);
    assert_eq!(result["panBounds"]["x"], 100.0);
    assert_eq!(result["panBounds"]["y"], 50.0);
    assert_eq!(result["zoomPoint"]["scale"], 2.0);
    assert_eq!(result["zoomPoint"]["x"], -50.0);
    assert_eq!(result["zoomPoint"]["y"], -20.0);
    assert_eq!(result["zoomReset"]["scale"], 1.0);
    assert_eq!(result["zoomReset"]["x"], 0.0);
    assert_eq!(result["zoomReset"]["y"], 0.0);
    assert_eq!(result["thresholdWide"], 120.0);
    assert_eq!(result["thresholdNarrow"], 40.0);
    assert_eq!(result["swipeYes"], true);
    assert_eq!(result["swipeNo"], false);
    assert_eq!(result["swipeSmall"], false);
    assert_eq!(result["tileName"], "a.jpg");
    assert_eq!(result["tileCaptured"], "2024-05-01 10:00:00");
    assert_eq!(result["tileAlt"], "a.jpg");
    assert_eq!(result["tileWidth"], 4000.0);
    assert_eq!(result["tileHeight"], 3000.0);
    assert_eq!(result["fitWide"]["width"], 1000.0);
    assert_eq!(result["fitWide"]["height"], 750.0);
    assert_eq!(result["fitTall"]["width"], 600.0);
    assert_eq!(result["fitTall"]["height"], 800.0);
    assert_eq!(result["fitMissing"], Value::Null);
    assert_eq!(result["placeholderWide"]["width"], 750.0);
    assert_eq!(result["placeholderWide"]["height"], 750.0);
    assert_eq!(result["placeholderTall"]["width"], 600.0);
    assert_eq!(result["placeholderTall"]["height"], 600.0);
    assert_eq!(result["placeholderMissing"], Value::Null);
    assert_eq!(result["originalDelay"], 300);
}
