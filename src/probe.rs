use std::ffi::OsStr;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, NaiveDateTime};
use exif::{In, Tag, Value};
use image::ImageReader;
use serde::{Deserialize, Serialize};

/// A reduced exposure time. Kept exact so repeated builds serialize the same
/// fraction and the viewer can format sub-second exposures faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exposure {
    pub numerator: u32,
    pub denominator: u32,
}

impl Exposure {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        (numerator > 0 && denominator > 0).then_some(Self {
            numerator,
            denominator,
        })
    }

    pub fn parse(value: &str) -> Option<Self> {
        let (numerator, denominator) = value.trim().split_once('/')?;
        Self::new(numerator.parse().ok()?, denominator.parse().ok()?)
    }

    /// Reduced `num/den` form used in shard records.
    pub fn fraction(self) -> String {
        let divisor = gcd(self.numerator, self.denominator);
        format!(
            "{}/{}",
            self.numerator / divisor,
            self.denominator / divisor
        )
    }

    pub fn seconds(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Metadata extracted from an image or video, with no location or private camera
/// fields beyond the values the catalog publishes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProbeResult {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_ms: Option<u64>,
    pub captured_at: Option<String>,
    pub model: Option<String>,
    pub f_number: Option<f32>,
    pub exposure: Option<Exposure>,
    pub iso: Option<u32>,
    pub focal_length_mm: Option<f32>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// EXIF values the catalog is allowed to keep.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExifMetadata {
    pub orientation: Option<u16>,
    pub captured_at: Option<String>,
    pub model: Option<String>,
    pub f_number: Option<f32>,
    pub exposure: Option<Exposure>,
    pub iso: Option<u32>,
    pub focal_length_mm: Option<f32>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
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

/// Probe a raster image for upright dimensions, EXIF metadata, and capture time.
/// Falls back to the file modification time when no capture timestamp exists.
pub fn probe_image(path: &Path, mtime_ns: i64) -> Result<ProbeResult> {
    let metadata = read_exif_metadata(path).unwrap_or_default();
    let reader = ImageReader::open(path)
        .with_context(|| format!("opening image {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("detecting format for {}", path.display()))?;
    let (raw_width, raw_height) = reader
        .into_dimensions()
        .with_context(|| format!("reading dimensions for {}", path.display()))?;
    let orientation = metadata.orientation.unwrap_or(1);
    let (width, height) = upright_dimensions(raw_width, raw_height, orientation);

    Ok(ProbeResult {
        width: Some(width),
        height: Some(height),
        duration_ms: None,
        captured_at: Some(metadata_capture_time(&metadata, mtime_ns)),
        model: metadata.model,
        f_number: metadata.f_number,
        exposure: metadata.exposure,
        iso: metadata.iso,
        focal_length_mm: metadata.focal_length_mm,
        latitude: metadata.latitude,
        longitude: metadata.longitude,
    })
}

/// Probe image metadata without decoding pixels. Used for containers such as
/// HEIC that the v1 raster decoder does not support.
pub fn probe_image_metadata(path: &Path, mtime_ns: i64) -> Result<ProbeResult> {
    let metadata = read_exif_metadata(path).unwrap_or_default();
    let orientation = metadata.orientation.unwrap_or(1);
    let (width, height) = match (metadata.pixel_width, metadata.pixel_height) {
        (Some(width), Some(height)) => {
            let (width, height) = upright_dimensions(width, height, orientation);
            (Some(width), Some(height))
        }
        _ => (None, None),
    };

    Ok(ProbeResult {
        width,
        height,
        duration_ms: None,
        captured_at: Some(metadata_capture_time(&metadata, mtime_ns)),
        model: metadata.model,
        f_number: metadata.f_number,
        exposure: metadata.exposure,
        iso: metadata.iso,
        focal_length_mm: metadata.focal_length_mm,
        latitude: metadata.latitude,
        longitude: metadata.longitude,
    })
}

fn metadata_capture_time(metadata: &ExifMetadata, mtime_ns: i64) -> String {
    metadata
        .captured_at
        .clone()
        .unwrap_or_else(|| timestamp_from_nanos(mtime_ns))
}

/// Probe a video through ffprobe, extracting upright dimensions, duration,
/// location tags, and a fallback capture time.
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
    let (latitude, longitude) = video_location(&json, video_stream).unwrap_or((None, None));

    Ok(ProbeResult {
        width,
        height,
        duration_ms,
        captured_at: Some(captured_at),
        model: video_model(&json, video_stream),
        f_number: None,
        exposure: None,
        iso: None,
        focal_length_mm: None,
        latitude,
        longitude,
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

/// Pick a camera model from common ffprobe tags, preferring format over stream
/// tags and QuickTime's namespaced key over the generic one.
fn video_model(json: &serde_json::Value, stream: Option<&serde_json::Value>) -> Option<String> {
    let tag_sets = [
        json.get("format").and_then(|format| format.get("tags")),
        stream.and_then(|stream| stream.get("tags")),
    ];
    for tags in tag_sets.into_iter().flatten() {
        for key in ["com.apple.quicktime.model", "model"] {
            if let Some(raw) = tags.get(key).and_then(|value| value.as_str()) {
                if let Some(model) = normalize_model(raw) {
                    return Some(model);
                }
            }
        }
    }
    None
}

/// Read Apple QuickTime and generic location tags from a probe result.
fn video_location(
    json: &serde_json::Value,
    stream: Option<&serde_json::Value>,
) -> Option<(Option<f64>, Option<f64>)> {
    let tag_sets = [
        json.get("format").and_then(|format| format.get("tags")),
        stream.and_then(|stream| stream.get("tags")),
    ];
    for tags in tag_sets.into_iter().flatten() {
        for key in ["com.apple.quicktime.location.ISO6709", "location"] {
            let Some(raw) = tags.get(key).and_then(|value| value.as_str()) else {
                continue;
            };
            if let Some((latitude, longitude)) = parse_location_tag(raw) {
                return Some((Some(latitude), Some(longitude)));
            }
        }
    }
    None
}

fn parse_location_tag(raw: &str) -> Option<(f64, f64)> {
    if let Some((latitude, longitude)) = raw.split_once(',') {
        if let (Ok(latitude), Ok(longitude)) = (
            latitude.trim().parse::<f64>(),
            longitude.trim().parse::<f64>(),
        ) {
            return validated_coordinates(latitude, longitude);
        }
    }
    parse_iso6709(raw)
}

/// Parse the leading latitude and longitude from an ISO 6709 value such as
/// `+52.5200+013.4050+036.500/`.
fn parse_iso6709(raw: &str) -> Option<(f64, f64)> {
    let text = raw.trim().trim_end_matches('/');
    let mut values = Vec::new();
    let mut start = 0usize;
    let bytes = text.as_bytes();
    for index in 1..bytes.len() {
        if bytes[index] == b'+' || bytes[index] == b'-' {
            if let Ok(value) = text[start..index].parse::<f64>() {
                values.push(value);
            } else {
                return None;
            }
            start = index;
        }
    }
    values.push(text[start..].parse::<f64>().ok()?);
    if values.len() < 2 {
        return None;
    }
    validated_coordinates(values[0], values[1])
}

fn validated_coordinates(latitude: f64, longitude: f64) -> Option<(f64, f64)> {
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
        || (latitude == 0.0 && longitude == 0.0)
    {
        return None;
    }
    Some((round_coordinate(latitude), round_coordinate(longitude)))
}

fn round_coordinate(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
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

/// Read the EXIF fields the catalog is allowed to use from a supported image
/// container.
pub fn read_exif_metadata(path: &Path) -> Option<ExifMetadata> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    Some(metadata_from_exif(&exif))
}

fn metadata_from_exif(exif: &exif::Exif) -> ExifMetadata {
    let mut metadata = ExifMetadata::default();

    if let Some(orientation) =
        field(exif, Tag::Orientation).and_then(|field| field.value.get_uint(0))
    {
        if (1..=8).contains(&orientation) {
            metadata.orientation = Some(orientation as u16);
        }
    }

    metadata.model = ascii_field(exif, Tag::Model).and_then(|raw| normalize_model(&raw));

    let timestamp = ascii_field(exif, Tag::DateTimeOriginal)
        .or_else(|| ascii_field(exif, Tag::DateTimeDigitized))
        .or_else(|| ascii_field(exif, Tag::DateTime));
    metadata.captured_at = timestamp.and_then(|raw| normalize_exif_timestamp(&raw));

    metadata.f_number = rational_field(exif, Tag::FNumber)
        .map(|value| round_one_decimal(value as f32))
        .filter(|value| *value > 0.0);

    metadata.exposure = exposure_field(exif, Tag::ExposureTime);

    metadata.iso = field(exif, Tag::PhotographicSensitivity)
        .and_then(|field| field.value.get_uint(0))
        .filter(|value| *value > 0);

    metadata.focal_length_mm = rational_field(exif, Tag::FocalLength)
        .map(|value| round_one_decimal(value as f32))
        .filter(|value| *value > 0.0);

    let latitude = gps_coordinate(exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)
        .filter(|value| (-90.0..=90.0).contains(value));
    let longitude = gps_coordinate(exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)
        .filter(|value| (-180.0..=180.0).contains(value));
    if let (Some(latitude), Some(longitude)) = (latitude, longitude) {
        if !(latitude == 0.0 && longitude == 0.0) {
            metadata.latitude = Some(latitude);
            metadata.longitude = Some(longitude);
        }
    }

    metadata.pixel_width =
        field(exif, Tag::PixelXDimension).and_then(|field| field.value.get_uint(0));
    metadata.pixel_height =
        field(exif, Tag::PixelYDimension).and_then(|field| field.value.get_uint(0));

    metadata
}

fn field(exif: &exif::Exif, tag: Tag) -> Option<&exif::Field> {
    exif.fields()
        .find(|field| field.tag == tag && field.ifd_num == In::PRIMARY)
        .or_else(|| exif.fields().find(|field| field.tag == tag))
}

fn ascii_field(exif: &exif::Exif, tag: Tag) -> Option<String> {
    let value = match &field(exif, tag)?.value {
        Value::Ascii(values) => values.first()?,
        _ => return None,
    };
    let end = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    let text = String::from_utf8_lossy(&value[..end]).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn rational_field(exif: &exif::Exif, tag: Tag) -> Option<f64> {
    rational_value(&field(exif, tag)?.value)
}

fn rational_value(value: &Value) -> Option<f64> {
    match value {
        Value::Rational(values) => {
            let first = values.first()?;
            (first.denom != 0).then_some(first.num as f64 / first.denom as f64)
        }
        Value::SRational(values) => {
            let first = values.first()?;
            (first.denom != 0).then_some(first.num as f64 / first.denom as f64)
        }
        _ => None,
    }
}

fn exposure_field(exif: &exif::Exif, tag: Tag) -> Option<Exposure> {
    match &field(exif, tag)?.value {
        Value::Rational(values) => {
            let first = values.first()?;
            Exposure::new(first.num, first.denom)
        }
        Value::SRational(values) => {
            let first = values.first()?;
            (first.num > 0 && first.denom > 0).then_some(Exposure {
                numerator: first.num as u32,
                denominator: first.denom as u32,
            })
        }
        _ => None,
    }
}

fn gps_coordinate(exif: &exif::Exif, value_tag: Tag, reference_tag: Tag) -> Option<f64> {
    let values = match &field(exif, value_tag)?.value {
        Value::Rational(values) if values.len() >= 3 => values,
        _ => return None,
    };
    let mut parts = [0.0; 3];
    for (index, rational) in values.iter().take(3).enumerate() {
        if rational.denom == 0 {
            return None;
        }
        parts[index] = rational.num as f64 / rational.denom as f64;
    }
    let mut decimal = parts[0] + parts[1] / 60.0 + parts[2] / 3600.0;
    let reference = ascii_field(exif, reference_tag).unwrap_or_default();
    if reference.eq_ignore_ascii_case("S") || reference.eq_ignore_ascii_case("W") {
        decimal = -decimal.abs();
    }
    decimal.is_finite().then_some(round_coordinate(decimal))
}

fn round_one_decimal(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

/// Collapse whitespace so a model string always fits on one caption line.
fn normalize_model(raw: &str) -> Option<String> {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_exif_timestamp(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let parsed = NaiveDateTime::parse_from_str(trimmed, "%Y:%m:%d %H:%M:%S").ok()?;
    Some(parsed.format("%Y-%m-%d %H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use exif::Rational;
    use std::io::Write;

    type Gps = (char, [(u32, u32); 3], char, [(u32, u32); 3]);

    #[derive(Clone, Copy, Default)]
    struct TestExif<'a> {
        orientation: u16,
        datetime: Option<&'a str>,
        model: Option<&'a str>,
        aperture: Option<(u32, u32)>,
        exposure: Option<(u32, u32)>,
        iso: Option<u16>,
        focal: Option<(u32, u32)>,
        pixel: Option<(u32, u32)>,
        gps: Option<Gps>,
    }

    fn write_plain_jpeg(path: &Path, width: u32, height: u32) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let image = image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 255) as u8, (y % 255) as u8, 128])
        });
        image.save(path).unwrap();
    }

    fn push_entry(out: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32) {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn inline_ascii(text: &str) -> u32 {
        let mut bytes = [0u8; 4];
        for (index, byte) in text.as_bytes().iter().take(4).enumerate() {
            bytes[index] = *byte;
        }
        u32::from_le_bytes(bytes)
    }

    fn push_rational(out: &mut Vec<u8>, value: (u32, u32)) {
        out.extend_from_slice(&value.0.to_le_bytes());
        out.extend_from_slice(&value.1.to_le_bytes());
    }

    fn tiff_with(fields: TestExif<'_>) -> Vec<u8> {
        let model_bytes = fields.model.map(|text| {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            bytes
        });
        let datetime_bytes = fields.datetime.map(|text| {
            let mut bytes = text.as_bytes().to_vec();
            bytes.push(0);
            bytes
        });
        let has_exif = datetime_bytes.is_some()
            || fields.aperture.is_some()
            || fields.exposure.is_some()
            || fields.iso.is_some()
            || fields.focal.is_some()
            || fields.pixel.is_some();
        let has_gps = fields.gps.is_some();

        let ifd0_count = 1
            + model_bytes.is_some() as usize
            + datetime_bytes.is_some() as usize
            + has_exif as usize
            + has_gps as usize;
        let ifd0_size = 2 + ifd0_count * 12 + 4;
        let exif_count = fields.aperture.is_some() as usize
            + fields.exposure.is_some() as usize
            + fields.iso.is_some() as usize
            + datetime_bytes.is_some() as usize
            + fields.focal.is_some() as usize
            + fields.pixel.is_some() as usize * 2;
        let exif_size = if has_exif { 2 + exif_count * 12 + 4 } else { 0 };
        let gps_count = 4;
        let gps_size = if has_gps { 2 + gps_count * 12 + 4 } else { 0 };

        let exif_offset = 8 + ifd0_size;
        let gps_offset = exif_offset + exif_size;
        let data_offset = gps_offset + gps_size;

        let mut cursor = data_offset;
        let model_offset = model_bytes.as_ref().map(|bytes| {
            let offset = cursor;
            cursor += bytes.len();
            offset
        });
        let datetime_offset = datetime_bytes.as_ref().map(|bytes| {
            let offset = cursor;
            cursor += bytes.len();
            offset
        });
        let aperture_offset = fields.aperture.map(|_| {
            let offset = cursor;
            cursor += 8;
            offset
        });
        let exposure_offset = fields.exposure.map(|_| {
            let offset = cursor;
            cursor += 8;
            offset
        });
        let focal_offset = fields.focal.map(|_| {
            let offset = cursor;
            cursor += 8;
            offset
        });
        let gps_latitude_offset = fields.gps.map(|_| {
            let offset = cursor;
            cursor += 24;
            offset
        });
        let gps_longitude_offset = fields.gps.map(|_| {
            let offset = cursor;
            cursor += 24;
            offset
        });

        let mut data = Vec::new();
        data.extend_from_slice(b"II");
        data.extend_from_slice(&42u16.to_le_bytes());
        data.extend_from_slice(&8u32.to_le_bytes());

        data.extend_from_slice(&(ifd0_count as u16).to_le_bytes());
        if let Some(bytes) = &model_bytes {
            let value = if bytes.len() <= 4 {
                inline_ascii(fields.model.unwrap())
            } else {
                model_offset.unwrap() as u32
            };
            push_entry(&mut data, 0x0110, 2, bytes.len() as u32, value);
        }
        push_entry(&mut data, 0x0112, 3, 1, fields.orientation.max(1) as u32);
        if let Some(bytes) = &datetime_bytes {
            let value = if bytes.len() <= 4 {
                inline_ascii(fields.datetime.unwrap())
            } else {
                datetime_offset.unwrap() as u32
            };
            push_entry(&mut data, 0x0132, 2, bytes.len() as u32, value);
        }
        if has_exif {
            push_entry(&mut data, 0x8769, 4, 1, exif_offset as u32);
        }
        if has_gps {
            push_entry(&mut data, 0x8825, 4, 1, gps_offset as u32);
        }
        data.extend_from_slice(&0u32.to_le_bytes());

        if has_exif {
            data.extend_from_slice(&(exif_count as u16).to_le_bytes());
            if let Some(value) = fields.exposure {
                push_entry(&mut data, 0x829A, 5, 1, exposure_offset.unwrap() as u32);
                let _ = value;
            }
            if let Some(value) = fields.aperture {
                push_entry(&mut data, 0x829D, 5, 1, aperture_offset.unwrap() as u32);
                let _ = value;
            }
            if let Some(value) = fields.iso {
                push_entry(&mut data, 0x8827, 3, 1, value as u32);
            }
            if let Some(bytes) = &datetime_bytes {
                push_entry(
                    &mut data,
                    0x9003,
                    2,
                    bytes.len() as u32,
                    datetime_offset.unwrap() as u32,
                );
            }
            if let Some(value) = fields.focal {
                push_entry(&mut data, 0x920A, 5, 1, focal_offset.unwrap() as u32);
                let _ = value;
            }
            if let Some((width, height)) = fields.pixel {
                push_entry(&mut data, 0xA002, 4, 1, width);
                push_entry(&mut data, 0xA003, 4, 1, height);
            }
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        if let Some((latitude_ref, _latitude, longitude_ref, _longitude)) = fields.gps {
            data.extend_from_slice(&(gps_count as u16).to_le_bytes());
            push_entry(
                &mut data,
                0x0001,
                2,
                2,
                inline_ascii(&format!("{latitude_ref}\0")),
            );
            push_entry(&mut data, 0x0002, 5, 3, gps_latitude_offset.unwrap() as u32);
            push_entry(
                &mut data,
                0x0003,
                2,
                2,
                inline_ascii(&format!("{longitude_ref}\0")),
            );
            push_entry(
                &mut data,
                0x0004,
                5,
                3,
                gps_longitude_offset.unwrap() as u32,
            );
            data.extend_from_slice(&0u32.to_le_bytes());
        }

        debug_assert_eq!(data.len(), data_offset);
        if let (Some(bytes), Some(offset)) = (&model_bytes, model_offset) {
            debug_assert_eq!(data.len(), offset);
            data.extend_from_slice(bytes);
        }
        if let (Some(bytes), Some(offset)) = (&datetime_bytes, datetime_offset) {
            debug_assert_eq!(data.len(), offset);
            data.extend_from_slice(bytes);
        }
        if let (Some(value), Some(offset)) = (fields.aperture, aperture_offset) {
            debug_assert_eq!(data.len(), offset);
            push_rational(&mut data, value);
        }
        if let (Some(value), Some(offset)) = (fields.exposure, exposure_offset) {
            debug_assert_eq!(data.len(), offset);
            push_rational(&mut data, value);
        }
        if let (Some(value), Some(offset)) = (fields.focal, focal_offset) {
            debug_assert_eq!(data.len(), offset);
            push_rational(&mut data, value);
        }
        if let Some((_, latitude, _, _)) = fields.gps {
            debug_assert_eq!(data.len(), gps_latitude_offset.unwrap());
            for value in latitude {
                push_rational(&mut data, value);
            }
        }
        if let Some((_, _, _, longitude)) = fields.gps {
            debug_assert_eq!(data.len(), gps_longitude_offset.unwrap());
            for value in longitude {
                push_rational(&mut data, value);
            }
        }
        data
    }

    fn jpeg_with_exif(path: &Path, fields: TestExif<'_>) {
        let temp = tempfile::tempdir().unwrap();
        let plain = temp.path().join("plain.jpg");
        write_plain_jpeg(&plain, 40, 20);
        let jpeg = std::fs::read(plain).unwrap();
        let tiff = tiff_with(fields);
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

    fn exif_image(fields: TestExif<'_>) -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("tagged.jpg");
        jpeg_with_exif(&path, fields);
        (temp, path)
    }

    #[test]
    fn parses_orientation_and_capture_time() {
        let (_temp, path) = exif_image(TestExif {
            orientation: 6,
            datetime: Some("2024:05:01 10:11:12"),
            ..TestExif::default()
        });
        let metadata = read_exif_metadata(&path).unwrap();
        assert_eq!(metadata.orientation, Some(6));
        assert_eq!(metadata.captured_at.as_deref(), Some("2024-05-01 10:11:12"));
        assert_eq!(metadata.model, None);
    }

    #[test]
    fn parses_and_normalizes_model() {
        let (_temp, path) = exif_image(TestExif {
            orientation: 1,
            datetime: Some("2024:05:01 10:11:12"),
            model: Some("  iPhone   15 Pro  "),
            ..TestExif::default()
        });
        let metadata = read_exif_metadata(&path).unwrap();
        assert_eq!(metadata.model.as_deref(), Some("iPhone 15 Pro"));

        let result = probe_image(&path, 0).unwrap();
        assert_eq!(result.model.as_deref(), Some("iPhone 15 Pro"));
    }

    #[test]
    fn empty_model_is_absent() {
        let (_temp, path) = exif_image(TestExif {
            orientation: 1,
            model: Some("   "),
            ..TestExif::default()
        });
        let metadata = read_exif_metadata(&path).unwrap();
        assert_eq!(metadata.model, None);
    }

    #[test]
    fn parses_camera_settings_and_gps() {
        let (_temp, path) = exif_image(TestExif {
            orientation: 1,
            datetime: Some("2024:05:01 10:11:12"),
            aperture: Some((18, 10)),
            exposure: Some((1, 125)),
            iso: Some(100),
            focal: Some((26, 1)),
            gps: Some((
                'N',
                [(52, 1), (30, 1), (0, 1)],
                'E',
                [(13, 1), (24, 1), (18, 1)],
            )),
            ..TestExif::default()
        });
        let metadata = read_exif_metadata(&path).unwrap();
        assert_eq!(metadata.f_number, Some(1.8));
        assert_eq!(metadata.exposure, Exposure::new(1, 125));
        assert_eq!(metadata.exposure.unwrap().fraction(), "1/125");
        assert_eq!(metadata.iso, Some(100));
        assert_eq!(metadata.focal_length_mm, Some(26.0));
        assert_eq!(metadata.latitude, Some(52.5));
        assert_eq!(metadata.longitude, Some(13.405));
    }

    #[test]
    fn south_west_references_are_negative() {
        let (_temp, path) = exif_image(TestExif {
            orientation: 1,
            gps: Some((
                'S',
                [(33, 1), (52, 1), (0, 1)],
                'W',
                [(70, 1), (30, 1), (0, 1)],
            )),
            ..TestExif::default()
        });
        let metadata = read_exif_metadata(&path).unwrap();
        assert_eq!(metadata.latitude, Some(-33.866667));
        assert_eq!(metadata.longitude, Some(-70.5));
    }

    #[test]
    fn invalid_gps_is_absent() {
        let value = Value::Rational(vec![
            Rational { num: 0, denom: 1 },
            Rational { num: 0, denom: 1 },
            Rational { num: 0, denom: 1 },
        ]);
        assert_eq!(rational_value(&value), Some(0.0));
        assert!(rational_value(&Value::Rational(vec![Rational { num: 1, denom: 0 }])).is_none());

        let (_temp, path) = exif_image(TestExif {
            orientation: 1,
            gps: Some(('N', [(0, 1), (0, 1), (0, 1)], 'E', [(0, 1), (0, 1), (0, 1)])),
            ..TestExif::default()
        });
        let metadata = read_exif_metadata(&path).unwrap();
        assert_eq!(metadata.latitude, None);
        assert_eq!(metadata.longitude, None);
    }

    #[test]
    fn metadata_only_probe_reads_pixel_dimensions() {
        let (_temp, path) = exif_image(TestExif {
            orientation: 6,
            datetime: Some("2024:05:01 10:11:12"),
            pixel: Some((4032, 3024)),
            ..TestExif::default()
        });
        let result = probe_image_metadata(&path, 0).unwrap();
        assert_eq!(result.width, Some(3024));
        assert_eq!(result.height, Some(4032));
        assert_eq!(result.captured_at.as_deref(), Some("2024-05-01 10:11:12"));
    }

    #[test]
    fn reads_model_from_video_tags_without_private_fields() {
        let json = serde_json::json!({
            "format": {
                "tags": {
                    "com.apple.quicktime.model": "  iPhone   15 Pro ",
                    "gps": "52.5,13.4",
                    "serial": "ABC123"
                }
            }
        });
        assert_eq!(video_model(&json, None).as_deref(), Some("iPhone 15 Pro"));

        let stream = serde_json::json!({"tags": {"model": "Canon EOS R6"}});
        assert_eq!(
            video_model(&serde_json::json!({}), Some(&stream)).as_deref(),
            Some("Canon EOS R6")
        );

        let none = serde_json::json!({"format": {"tags": {"gps": "52.5"}}});
        assert_eq!(video_model(&none, None), None);
    }

    #[test]
    fn parses_video_location_tags() {
        assert_eq!(parse_iso6709("+52.5200+013.4050/"), Some((52.52, 13.405)));
        assert_eq!(parse_location_tag("52.5200,13.4050"), Some((52.52, 13.405)));
        assert_eq!(parse_iso6709("not a location"), None);
        assert_eq!(parse_location_tag("0,0"), None);
        assert_eq!(parse_location_tag("91,13"), None);

        let json = serde_json::json!({
            "format": {"tags": {"com.apple.quicktime.location.ISO6709": "+52.5200+013.4050/"}}
        });
        assert_eq!(
            video_location(&json, None),
            Some((Some(52.52), Some(13.405)))
        );
    }

    #[test]
    fn untagged_image_falls_back_to_mtime() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("plain.jpg");
        write_plain_jpeg(&path, 40, 20);
        let metadata = read_exif_metadata(&path).unwrap_or_default();
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
        let (_temp, path) = exif_image(TestExif {
            orientation: 6,
            datetime: Some("2024:05:01 10:11:12"),
            ..TestExif::default()
        });
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
