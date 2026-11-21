mod common;

use common::Fixture;
use media_catalog::scan::scan_media;

#[test]
fn scan_fixture_reports_count_grouping_and_ignored_files() {
    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 40, 20, 1);
    fixture.write_png("2024/05/nested/b.png", 30, 30);
    fixture.write_bytes("2024/06/c.mp4", b"not a real video");
    fixture.write_bytes("2023/12/d.webp", b"not a real webp");
    fixture.write_bytes("2024/05/notes.txt", b"skip");
    fixture.write_bytes("2024/05/.hidden.jpg", b"skip");
    fixture.write_bytes("2024/05/sidecar.xmp", b"skip");
    fixture.write_image("loose.jpg", 10, 10, 2);

    let result = scan_media(&fixture.media).unwrap();
    let paths: Vec<_> = result
        .files
        .iter()
        .map(|file| file.rel_path.as_str())
        .collect();
    assert_eq!(
        paths,
        vec![
            "2023/12/d.webp",
            "2024/05/a.jpg",
            "2024/05/nested/b.png",
            "2024/06/c.mp4"
        ]
    );
    assert_eq!(result.files[1].year, 2024);
    assert_eq!(result.files[1].month, 5);
    assert_eq!(result.files[2].month, 5);
    assert_eq!(result.files[3].month, 6);
    assert!(result.errors.is_empty());
}

#[cfg(unix)]
#[test]
fn scan_reports_unreadable_directories_without_aborting() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    fixture.write_image("2024/05/a.jpg", 10, 10, 1);
    let private = fixture.media.join("2024/06");
    std::fs::create_dir_all(&private).unwrap();
    std::fs::write(private.join("b.jpg"), b"x").unwrap();
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();

    let result = scan_media(&fixture.media).unwrap();
    let paths: Vec<_> = result
        .files
        .iter()
        .map(|file| file.rel_path.as_str())
        .collect();
    assert_eq!(paths, vec!["2024/05/a.jpg"]);
    assert!(!result.errors.is_empty());
    assert!(result.errors[0].path.contains("2024/06"));

    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();
}
