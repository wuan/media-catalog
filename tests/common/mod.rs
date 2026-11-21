#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use media_catalog::config::Config;
use media_catalog::probe;

/// A temporary media tree plus output and state locations.
pub struct Fixture {
    temp: TempDir,
    pub media: PathBuf,
    pub public: PathBuf,
    pub state: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let media = temp.path().join("media");
        let public = temp.path().join("public");
        let state = temp.path().join("state.sqlite");
        std::fs::create_dir_all(&media).expect("media dir");
        Self {
            temp,
            media,
            public,
            state,
        }
    }

    pub fn config(&self) -> Config {
        let mut config = Config::with_roots(&self.media, self.root());
        config.state_db = self.state.clone();
        config.jobs = 2;
        config.quiet = true;
        config
    }

    pub fn root(&self) -> &Path {
        self.temp.path()
    }

    pub fn media_path(&self, rel: &str) -> PathBuf {
        self.media.join(rel)
    }

    pub fn public_path(&self, rel: &str) -> PathBuf {
        self.public.join(rel)
    }

    pub fn write_bytes(&self, rel: &str, bytes: &[u8]) {
        let path = self.media_path(rel);
        ensure_parent(&path);
        std::fs::write(path, bytes).expect("write fixture file");
    }

    pub fn write_image(&self, rel: &str, width: u32, height: u32, seed: u8) {
        let path = self.media_path(rel);
        ensure_parent(&path);
        let image = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([
                ((x + seed as u32) % 256) as u8,
                ((y + seed as u32) % 256) as u8,
                120,
            ])
        });
        image.save(&path).expect("write fixture image");
    }

    pub fn write_png(&self, rel: &str, width: u32, height: u32) {
        let path = self.media_path(rel);
        ensure_parent(&path);
        image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 200])
        })
        .save(&path)
        .expect("write fixture png");
    }

    pub fn write_jpeg_with_exif(
        &self,
        rel: &str,
        orientation: Option<u16>,
        datetime: Option<&str>,
        gps: bool,
    ) {
        let path = self.media_path(rel);
        ensure_parent(&path);
        let plain = self.temp.path().join("plain-for-exif.jpg");
        image::RgbImage::from_fn(40, 20, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        })
        .save(&plain)
        .expect("plain jpeg");
        let jpeg = std::fs::read(&plain).expect("plain jpeg bytes");
        let tiff = tiff_with(orientation, datetime, gps);
        let mut app1 = Vec::new();
        app1.extend_from_slice(&[0xFF, 0xE1]);
        app1.extend_from_slice(&((2 + 6 + tiff.len()) as u16).to_be_bytes());
        app1.extend_from_slice(b"Exif\0\0");
        app1.extend_from_slice(&tiff);

        let mut out = Vec::new();
        out.extend_from_slice(&jpeg[0..2]);
        out.extend_from_slice(&app1);
        out.extend_from_slice(&jpeg[2..]);
        std::fs::write(path, out).expect("write exif jpeg");
    }

    pub fn write_video(&self, rel: &str, seconds: u32) -> bool {
        if probe::ensure_external_tools().is_err() {
            return false;
        }
        let path = self.media_path(rel);
        ensure_parent(&path);
        Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("testsrc=size=64x48:rate=10:duration={seconds}"),
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&path)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    pub fn remove(&self, rel: &str) {
        std::fs::remove_file(self.media_path(rel)).expect("remove fixture file");
    }

    pub fn thumb_path(&self, rel: &str) -> PathBuf {
        self.public.join(format!("thumbs/{rel}.webp"))
    }

    pub fn thumb_mtime(&self, rel: &str) -> std::time::SystemTime {
        std::fs::metadata(self.thumb_path(rel))
            .expect("thumb metadata")
            .modified()
            .expect("thumb mtime")
    }

    pub fn read_json(&self, rel: &str) -> String {
        std::fs::read_to_string(self.public_path(rel)).expect("read json")
    }

    pub fn read_html(&self, rel: &str) -> String {
        std::fs::read_to_string(self.public_path(rel)).expect("read html")
    }
}

pub fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
}

/// Build a little-endian TIFF stream containing an orientation tag, an optional
/// DateTimeOriginal in the EXIF IFD, and an optional GPS IFD with coordinates.
fn tiff_with(orientation: Option<u16>, datetime: Option<&str>, gps: bool) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"II");
    data.extend_from_slice(&42u16.to_le_bytes());
    data.extend_from_slice(&8u32.to_le_bytes());

    let ifd0_entries = 1 + datetime.is_some() as usize + gps as usize;
    let ifd0_size = 2 + ifd0_entries * 12 + 4;
    let mut next_offset = 8 + ifd0_size;

    let exif_ifd_offset = if datetime.is_some() {
        let offset = next_offset;
        next_offset += 2 + 12 + 4;
        Some(offset)
    } else {
        None
    };

    let gps_ifd_offset = if gps {
        let offset = next_offset;
        next_offset += 2 + 2 * 12 + 4;
        Some(offset)
    } else {
        None
    };

    let datetime_offset = if let Some(_) = exif_ifd_offset {
        let offset = next_offset;
        next_offset += 20;
        Some(offset)
    } else {
        None
    };

    let rational_offset = if gps_ifd_offset.is_some() {
        Some(next_offset)
    } else {
        None
    };

    // IFD0
    data.extend_from_slice(&(ifd0_entries as u16).to_le_bytes());
    data.extend_from_slice(&0x0112u16.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&orientation.unwrap_or(1).to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

    if let Some(offset) = exif_ifd_offset {
        data.extend_from_slice(&0x8769u16.to_le_bytes());
        data.extend_from_slice(&4u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(offset as u32).to_le_bytes());
    }
    if let Some(offset) = gps_ifd_offset {
        data.extend_from_slice(&0x8825u16.to_le_bytes());
        data.extend_from_slice(&4u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(offset as u32).to_le_bytes());
    }
    data.extend_from_slice(&0u32.to_le_bytes());

    // Exif IFD with DateTimeOriginal
    if let (Some(_), Some(datetime_offset)) = (exif_ifd_offset, datetime_offset) {
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&0x9003u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&20u32.to_le_bytes());
        data.extend_from_slice(&(datetime_offset as u32).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
    }

    // GPS IFD with latitude reference (inline) and latitude rationals.
    if let (Some(_), Some(rational_offset)) = (gps_ifd_offset, rational_offset) {
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&0x0001u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&[b'N', 0, 0, 0]);
        data.extend_from_slice(&0x0002u16.to_le_bytes());
        data.extend_from_slice(&5u16.to_le_bytes());
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&(rational_offset as u32).to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
    }

    if let Some(datetime_offset) = datetime_offset {
        let mut text = datetime
            .unwrap_or("2024:05:01 10:11:12")
            .as_bytes()
            .to_vec();
        text.resize(20, 0);
        debug_assert_eq!(data.len(), datetime_offset);
        data.extend_from_slice(&text);
    }

    if let Some(rational_offset) = rational_offset {
        debug_assert_eq!(data.len(), rational_offset);
        for numerator in [52u32, 31, 12] {
            data.extend_from_slice(&numerator.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
        }
    }

    data
}

/// Set/unset read permission bits on Unix so a source cannot be opened.
#[cfg(unix)]
pub fn set_readable(path: &Path, readable: bool) {
    use std::os::unix::fs::PermissionsExt;
    let mode = if readable { 0o644 } else { 0o000 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("chmod");
}
