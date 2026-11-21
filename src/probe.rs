use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, NaiveDateTime};
use image::ImageReader;

/// Metadata extracted from an image or video, with no location or camera fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProbeResult {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub captured_at: Option<String>,
}

/// EXIF values the catalog is allowed to keep: orientation and capture time only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExifMetadata {
    pub orientation: Option<u16>,
    pub captured_at: Option<String>,
}

/// Check that `ffmpeg` and `ffprobe` are available before any scanning happens.
pub fn ensure_external_tools() -> Result<()> {
    check_external_tools(None)
}

/// Run the tool checks with an optional PATH override (used by tests to prove the
/// clear failure message when the tools are missing).
pub fn check_external_tools(path_env: Option<&OsStr>) -> Result<()> {
    for tool in ["ffmpeg", "ffprobe"] {
        let mut command = Command::new(tool);
        command.arg("-version");
        if let Some(path) = path_env {
            command.env("PATH", path);
        }
        let output = command.output().map_err(|error| {
            anyhow::anyhow!(
                "{tool} is required for video probing and poster extraction but could not be run: {error}. \
                 Install ffmpeg and ensure both ffmpeg and ffprobe are on PATH."
            )
        })?;
        if !output.status.success() {
            bail!(
                "{tool} is required but exited with {}. Install a working ffmpeg/ffprobe.",
                output.status
            );
        }
    }
    Ok(())
}

/// Probe a raster image for upright dimensions, EXIF orientation, and capture time.
/// Falls back to the file modification time when no capture timestamp exists.
pub fn probe_image(path: &Path, mtime_ns: i64) -> Result<ProbeResult> {
    let exif = read_jpeg_exif(path).unwrap_or_default();
    let reader = ImageReader::open(path)
        .with_context(|| format!("opening image {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("detecting format for {}", path.display()))?;
    let (raw_width, raw_height) = reader
        .into_dimensions()
        .with_context(|| format!("reading dimensions for {}", path.display()))?;
    let orientation = exif.orientation.unwrap_or(1);
    let (width, height) = upright_dimensions(raw_width, raw_height, orientation);

    Ok(ProbeResult {
        width: Some(width),
        height: Some(height),
        duration_ms: None,
        captured_at: Some(
            exif.captured_at
                .unwrap_or_else(|| timestamp_from_nanos(mtime_ns)),
        ),
    })
}

/// Probe a video through ffprobe, extracting upright dimensions, duration, and
/// a fallback capture time.
pub fn probe_video(path: &Path, mtime_ns: i64) -> Result<ProbeResult> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .with_context(|| format!("running ffprobe for {}", path.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe failed for {}: {}", path.display(), stderr.trim());
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing ffprobe JSON")?;
    let streams = json
        .get("streams")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let video_stream = streams
        .iter()
        .find(|stream| stream.get("codec_type").and_then(|v| v.as_str()) == Some("video"));

    let raw_width = video_stream
        .and_then(|stream| stream.get("width"))
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let raw_height = video_stream
        .and_then(|stream| stream.get("height"))
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let rotation = video_stream.map(video_rotation).unwrap_or(0.0);

    let (width, height) = match (raw_width, raw_height) {
        (Some(width), Some(height)) => {
            let swapped = (rotation.round() as i64).rem_euclid(180) == 90;
            if swapped {
                (Some(height), Some(width))
            } else {
                (Some(width), Some(height))
            }
        }
        _ => (raw_width, raw_height),
    };

    let duration_seconds = json
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<f64>().ok())
        .or_else(|| {
            video_stream
                .and_then(|stream| stream.get("duration"))
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<f64>().ok())
        });
    let duration_ms = duration_seconds
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map(|seconds| (seconds * 1000.0).round() as u64);

    let captured_at =
        creation_time(&json, video_stream).unwrap_or_else(|| timestamp_from_nanos(mtime_ns));

    Ok(ProbeResult {
        width,
        height,
        duration_ms,
        captured_at: Some(captured_at),
    })
}

fn creation_time(json: &serde_json::Value, stream: Option<&serde_json::Value>) -> Option<String> {
    let raw = json
        .get("format")
        .and_then(|format| format.get("tags"))
        .and_then(|tags| tags.get("creation_time"))
        .and_then(|value| value.as_str())
        .or_else(|| {
            stream
                .and_then(|stream| stream.get("tags"))
                .and_then(|tags| tags.get("creation_time"))
                .and_then(|value| value.as_str())
        })?;
    DateTime::parse_from_rfc3339(raw).ok().map(|timestamp| {
        timestamp
            .naive_local()
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
    })
}

fn video_rotation(stream: &serde_json::Value) -> f64 {
    let mut rotation = stream
        .get("tags")
        .and_then(|tags| tags.get("rotate"))
        .and_then(|value| {
            value
                .as_str()
                .and_then(|text| text.parse::<f64>().ok())
                .or_else(|| value.as_f64())
        });

    if rotation.is_none() {
        rotation = stream
            .get("side_data_list")
            .and_then(|value| value.as_array())
            .and_then(|entries| {
                entries.iter().find_map(|entry| {
                    entry.get("rotation").and_then(|value| {
                        value
                            .as_f64()
                            .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
                    })
                })
            });
    }
    rotation.unwrap_or(0.0)
}

/// Swap dimensions for EXIF orientations 5-8 (90/270 degree rotations).
pub fn upright_dimensions(width: u32, height: u32, orientation: u16) -> (u32, u32) {
    match orientation {
        5..=8 => (height, width),
        _ => (width, height),
    }
}

/// Format nanoseconds since the Unix epoch as a naive local timestamp.
///
/// Capture metadata such as EXIF has no timezone, so the catalog deliberately
/// stores zone-free wall-clock strings.
pub fn timestamp_from_nanos(mtime_ns: i64) -> String {
    if mtime_ns <= 0 {
        return "1970-01-01 00:00:00".to_string();
    }
    let seconds = (mtime_ns / 1_000_000_000) as u64;
    let nanos = (mtime_ns.rem_euclid(1_000_000_000)) as u32;
    let system_time = UNIX_EPOCH + Duration::new(seconds, nanos);
    let local: DateTime<Local> = system_time.into();
    local.naive_local().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Read the EXIF fields the catalog is allowed to use. Currently JPEG only;
/// other formats fall back to the file modification time.
pub fn read_exif_metadata(path: &Path) -> Option<ExifMetadata> {
    read_jpeg_exif(path)
}

/// Read a bounded prefix of the file, enough to find the JPEG APP1/EXIF segment.
fn read_jpeg_exif(path: &Path) -> Option<ExifMetadata> {
    let mut file = File::open(path).ok()?;
    let mut buffer = vec![0u8; 512 * 1024];
    let mut filled = 0;
    while filled < buffer.len() {
        match file.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(_) => return None,
        }
    }
    buffer.truncate(filled);
    parse_exif_from_jpeg(&buffer)
}

/// Parse the EXIF APP1 segment of a JPEG, extracting only orientation and capture time.
pub fn parse_exif_from_jpeg(bytes: &[u8]) -> Option<ExifMetadata> {
    let tiff = find_exif_segment(bytes)?;
    parse_tiff(tiff)
}

fn find_exif_segment(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut position = 2usize;
    while position + 4 <= bytes.len() {
        if bytes[position] != 0xFF {
            return None;
        }
        let marker = bytes[position + 1];
        position += 2;
        if marker == 0xD9 || marker == 0xDA {
            return None;
        }
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes([bytes[position], bytes[position + 1]]) as usize;
        if length < 2 || position + length > bytes.len() {
            return None;
        }
        let data = &bytes[position + 2..position + length];
        if marker == 0xE1 && data.starts_with(b"Exif\0\0") {
            return Some(&data[6..]);
        }
        position += length;
    }
    None
}

struct TiffReader<'a> {
    data: &'a [u8],
    little_endian: bool,
}

impl<'a> TiffReader<'a> {
    fn u16(&self, offset: usize) -> Option<u16> {
        let bytes = self.data.get(offset..offset + 2)?;
        Some(if self.little_endian {
            u16::from_le_bytes([bytes[0], bytes[1]])
        } else {
            u16::from_be_bytes([bytes[0], bytes[1]])
        })
    }

    fn u32(&self, offset: usize) -> Option<u32> {
        let bytes = self.data.get(offset..offset + 4)?;
        Some(if self.little_endian {
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        } else {
            u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        })
    }

    fn inline_short(&self, entry_offset: usize) -> Option<u16> {
        self.u16(entry_offset + 8)
    }

    fn inline_long(&self, entry_offset: usize) -> Option<u32> {
        self.u32(entry_offset + 8)
    }
}

fn parse_tiff(data: &[u8]) -> Option<ExifMetadata> {
    if data.len() < 8 {
        return None;
    }
    let little_endian = match &data[0..2] {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };
    let reader = TiffReader {
        data,
        little_endian,
    };
    if reader.u16(2)? != 42 {
        return None;
    }
    let ifd0_offset = reader.u32(4)? as usize;

    let mut metadata = ExifMetadata::default();
    let (orientation_entry, exif_ifd_entry, date_time_entry) =
        find_ifd0_entries(&reader, ifd0_offset)?;

    if let Some(entry) = orientation_entry {
        let orientation = reader.inline_short(entry)?;
        if (1..=8).contains(&orientation) {
            metadata.orientation = Some(orientation);
        }
    }

    let mut timestamp = None;
    if let Some(entry) = date_time_entry {
        timestamp = read_ascii_value(&reader, entry);
    }
    if let Some(entry) = exif_ifd_entry {
        let exif_offset = reader.inline_long(entry)? as usize;
        if let Some((date_time_original, date_time_digitized)) =
            find_exif_entries(&reader, exif_offset)
        {
            if let Some(entry) = date_time_original.or(date_time_digitized) {
                timestamp = read_ascii_value(&reader, entry).or(timestamp);
            }
        }
    }
    metadata.captured_at = timestamp.and_then(|raw| normalize_exif_timestamp(&raw));
    Some(metadata)
}

/// Returns (orientation entry offset, ExifIFD pointer entry offset, DateTime entry offset).
fn find_ifd0_entries(
    reader: &TiffReader<'_>,
    offset: usize,
) -> Option<(Option<usize>, Option<usize>, Option<usize>)> {
    let count = reader.u16(offset)? as usize;
    let entries_start = offset + 2;
    let mut orientation = None;
    let mut exif_ifd = None;
    let mut date_time = None;
    for index in 0..count {
        let entry = entries_start + index * 12;
        let tag = reader.u16(entry)?;
        match tag {
            0x0112 => orientation = Some(entry),
            0x8769 => exif_ifd = Some(entry),
            0x0132 => date_time = Some(entry),
            _ => {}
        }
    }
    Some((orientation, exif_ifd, date_time))
}

/// Returns (DateTimeOriginal entry offset, DateTimeDigitized entry offset).
fn find_exif_entries(
    reader: &TiffReader<'_>,
    offset: usize,
) -> Option<(Option<usize>, Option<usize>)> {
    let count = reader.u16(offset)? as usize;
    let entries_start = offset + 2;
    let mut date_time_original = None;
    let mut date_time_digitized = None;
    for index in 0..count {
        let entry = entries_start + index * 12;
        let tag = reader.u16(entry)?;
        match tag {
            0x9003 => date_time_original = Some(entry),
            0x9004 => date_time_digitized = Some(entry),
            _ => {}
        }
    }
    Some((date_time_original, date_time_digitized))
}

fn read_ascii_value(reader: &TiffReader<'_>, entry: usize) -> Option<String> {
    let entry_type = reader.u16(entry + 2)?;
    if entry_type != 2 {
        return None;
    }
    let count = reader.u32(entry + 4)? as usize;
    if count == 0 || count > 256 {
        return None;
    }
    let value_offset = if count <= 4 {
        entry + 8
    } else {
        reader.u32(entry + 8)? as usize
    };
    let bytes = reader.data.get(value_offset..value_offset + count)?;
    let text = bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .collect::<Vec<u8>>();
    Some(String::from_utf8_lossy(&text).to_string())
}

fn normalize_exif_timestamp(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let parsed = NaiveDateTime::parse_from_str(trimmed, "%Y:%m:%d %H:%M:%S").ok()?;
    Some(parsed.format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_plain_jpeg(path: &Path, width: u32, height: u32) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let image = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, 128])
        });
        image.save(path).unwrap();
    }

    fn tiff_with(orientation: Option<u16>, datetime: Option<&str>) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(b"II");
        data.extend_from_slice(&42u16.to_le_bytes());
        data.extend_from_slice(&8u32.to_le_bytes());

        let include_exif = datetime.is_some();
        let ifd0_entries = 1 + include_exif as usize;
        let ifd0_size = 2 + ifd0_entries * 12 + 4;
        let exif_ifd_offset = 8 + ifd0_size;
        let exif_ifd_size = 2 + 12 + 4;
        let datetime_offset = exif_ifd_offset + exif_ifd_size;

        data.extend_from_slice(&(ifd0_entries as u16).to_le_bytes());
        // Orientation, type SHORT, count 1, inline.
        data.extend_from_slice(&0x0112u16.to_le_bytes());
        data.extend_from_slice(&3u16.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&orientation.unwrap_or(1).to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        if include_exif {
            data.extend_from_slice(&0x8769u16.to_le_bytes());
            data.extend_from_slice(&4u16.to_le_bytes());
            data.extend_from_slice(&1u32.to_le_bytes());
            data.extend_from_slice(&(exif_ifd_offset as u32).to_le_bytes());
        }
        data.extend_from_slice(&0u32.to_le_bytes()); // next IFD

        if include_exif {
            data.extend_from_slice(&1u16.to_le_bytes());
            data.extend_from_slice(&0x9003u16.to_le_bytes());
            data.extend_from_slice(&2u16.to_le_bytes());
            data.extend_from_slice(&20u32.to_le_bytes());
            data.extend_from_slice(&(datetime_offset as u32).to_le_bytes());
            data.extend_from_slice(&0u32.to_le_bytes()); // next IFD
            let mut text = datetime.unwrap().as_bytes().to_vec();
            text.resize(20, 0);
            data.extend_from_slice(&text);
        }
        data
    }

    fn jpeg_with_exif(path: &Path, orientation: Option<u16>, datetime: Option<&str>) {
        let temp = tempfile::tempdir().unwrap();
        let plain = temp.path().join("plain.jpg");
        write_plain_jpeg(&plain, 40, 20);
        let jpeg = std::fs::read(plain).unwrap();
        let tiff = tiff_with(orientation, datetime);
        let mut app1 = Vec::new();
        app1.extend_from_slice(&[0xFF, 0xE1]);
        let length = (2 + 6 + tiff.len()) as u16;
        app1.extend_from_slice(&length.to_be_bytes());
        app1.extend_from_slice(b"Exif\0\0");
        app1.extend_from_slice(&tiff);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(&jpeg[0..2]).unwrap();
        file.write_all(&app1).unwrap();
        file.write_all(&jpeg[2..]).unwrap();
        file.flush().unwrap();
    }

    #[test]
    fn parses_orientation_and_capture_time() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tagged.jpg");
        jpeg_with_exif(&path, Some(6), Some("2024:05:01 10:11:12"));
        let metadata = read_jpeg_exif(&path).unwrap();
        assert_eq!(metadata.orientation, Some(6));
        assert_eq!(metadata.captured_at.as_deref(), Some("2024-05-01 10:11:12"));
    }

    #[test]
    fn untagged_image_falls_back_to_mtime() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("plain.jpg");
        write_plain_jpeg(&path, 40, 20);
        let metadata = read_jpeg_exif(&path).unwrap_or_default();
        assert_eq!(metadata.orientation, None);

        let result = probe_image(&path, 1_700_000_000_000_000_000).unwrap();
        assert_eq!(result.width, Some(40));
        assert_eq!(result.height, Some(20));
        assert_eq!(
            result.captured_at.as_deref(),
            Some(timestamp_from_nanos(1_700_000_000_000_000_000).as_str())
        );
    }

    #[test]
    fn rotated_tagged_image_reports_upright_dimensions() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("rotated.jpg");
        jpeg_with_exif(&path, Some(6), Some("2024:05:01 10:11:12"));
        let result = probe_image(&path, 0).unwrap();
        assert_eq!(result.width, Some(20));
        assert_eq!(result.height, Some(40));
        assert_eq!(result.captured_at.as_deref(), Some("2024-05-01 10:11:12"));
    }

    #[test]
    fn reads_video_rotation_from_tags_and_side_data() {
        let tagged: serde_json::Value = serde_json::json!({"tags": {"rotate": "-90"}});
        assert_eq!(video_rotation(&tagged), -90.0);
        let side_data: serde_json::Value = serde_json::json!({
            "side_data_list": [{"side_data_type": "Display Matrix", "rotation": 270}]
        });
        assert_eq!(video_rotation(&side_data), 270.0);
        let none: serde_json::Value = serde_json::json!({});
        assert_eq!(video_rotation(&none), 0.0);
    }

    #[test]
    fn orientation_dimension_swaps_only_for_rotations() {
        assert_eq!(upright_dimensions(4000, 3000, 1), (4000, 3000));
        assert_eq!(upright_dimensions(4000, 3000, 3), (4000, 3000));
        assert_eq!(upright_dimensions(4000, 3000, 6), (3000, 4000));
        assert_eq!(upright_dimensions(4000, 3000, 8), (3000, 4000));
    }

    #[test]
    fn missing_tools_produce_a_clear_error() {
        let error = check_external_tools(Some(OsStr::new("/nonexistent-path"))).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("ffmpeg is required"));
        assert!(message.contains("PATH"));
    }

    #[test]
    fn probes_video_when_ffmpeg_is_available() {
        if ensure_external_tools().is_err() {
            eprintln!("skipping video probe test: ffmpeg/ffprobe unavailable");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("clip.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=64x48:rate=10:duration=2",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        let result = probe_video(&path, 0).unwrap();
        assert_eq!(result.width, Some(64));
        assert_eq!(result.height, Some(48));
        let duration = result.duration_ms.unwrap();
        assert!((1900..=2200).contains(&duration), "duration was {duration}");
    }
}
