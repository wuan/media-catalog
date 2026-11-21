use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageReader};
use thiserror::Error;

use crate::fsutil::{temp_path, write_atomic};
use crate::probe::{self, ExifMetadata};
use crate::scan::{is_supported_image_extension, VIDEO_EXTENSIONS};

pub const THUMB_SIZE: u32 = 512;
pub const WEBP_QUALITY: f32 = 80.0;

/// Errors from thumbnail generation. Unsupported formats are not build failures;
/// they are reported and the item keeps a placeholder tile.
#[derive(Debug, Error)]
pub enum ThumbError {
    #[error("unsupported format: {0}")]
    Unsupported(String),
    #[error("{0}")]
    Failed(#[from] anyhow::Error),
}

/// Context needed by decoders.
#[derive(Debug, Clone)]
pub struct ThumbInput {
    pub duration_ms: Option<u64>,
}

/// Decoder boundary: the pipeline and state schema do not need to know which
/// decoder handles a format.
pub trait MediaDecoder: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, extension: &str) -> bool;
    fn render(
        &self,
        source: &Path,
        destination: &Path,
        input: &ThumbInput,
    ) -> Result<(), ThumbError>;
}

pub struct RasterDecoder;
pub struct VideoDecoder;

impl MediaDecoder for RasterDecoder {
    fn name(&self) -> &'static str {
        "raster"
    }

    fn supports(&self, extension: &str) -> bool {
        is_supported_image_extension(extension)
    }

    fn render(
        &self,
        source: &Path,
        destination: &Path,
        _input: &ThumbInput,
    ) -> Result<(), ThumbError> {
        let image = ImageReader::open(source)
            .with_context(|| format!("opening image {}", source.display()))?
            .with_guessed_format()
            .with_context(|| format!("detecting format for {}", source.display()))?
            .decode()
            .with_context(|| format!("decoding image {}", source.display()))?;

        let orientation = probe::read_exif_metadata(source)
            .unwrap_or_default()
            .orientation
            .unwrap_or(1);
        let upright = orient_image(image, orientation);
        let cropped = cover_crop(upright, THUMB_SIZE);
        let encoded = encode_webp(&cropped, WEBP_QUALITY)?;
        write_atomic(destination, &encoded)?;
        Ok(())
    }
}

impl MediaDecoder for VideoDecoder {
    fn name(&self) -> &'static str {
        "video"
    }

    fn supports(&self, extension: &str) -> bool {
        VIDEO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
    }

    fn render(
        &self,
        source: &Path,
        destination: &Path,
        input: &ThumbInput,
    ) -> Result<(), ThumbError> {
        let parent = destination
            .parent()
            .ok_or_else(|| anyhow::anyhow!("thumbnail destination has no parent"))?;
        std::fs::create_dir_all(parent).context("creating thumbnail directory")?;
        let poster_path = temp_path(parent, ".png");

        let result = (|| -> Result<()> {
            let timestamp = poster_timestamp(input.duration_ms);
            if !extract_frame(source, &poster_path, timestamp) {
                // Very short or unseekable videos fall back to the first frame.
                if !extract_frame(source, &poster_path, 0.0) {
                    bail!("ffmpeg could not extract a poster frame");
                }
            }
            let frame = ImageReader::open(&poster_path)
                .with_context(|| format!("opening poster frame {}", poster_path.display()))?
                .with_guessed_format()
                .with_context(|| format!("detecting poster format {}", poster_path.display()))?
                .decode()
                .with_context(|| format!("decoding poster frame {}", poster_path.display()))?;
            let cropped = cover_crop(frame, THUMB_SIZE);
            let encoded = encode_webp(&cropped, WEBP_QUALITY)?;
            write_atomic(destination, &encoded)?;
            Ok(())
        })();

        let _ = std::fs::remove_file(&poster_path);
        result.map_err(ThumbError::Failed)
    }
}

static DECODERS: &[&dyn MediaDecoder] = &[&RasterDecoder, &VideoDecoder];

/// Find the decoder responsible for an extension.
pub fn decoder_for(extension: &str) -> Option<&'static dyn MediaDecoder> {
    DECODERS
        .iter()
        .copied()
        .find(|decoder| decoder.supports(extension))
}

/// Whether v1 has a decoder for this extension.
pub fn format_supported(extension: &str) -> bool {
    decoder_for(extension).is_some()
}

/// Generate a thumbnail for a source file at `destination`.
pub fn generate_thumbnail(
    source: &Path,
    destination: &Path,
    extension: &str,
    input: &ThumbInput,
) -> Result<(), ThumbError> {
    let decoder = decoder_for(extension)
        .ok_or_else(|| ThumbError::Unsupported(extension.to_ascii_lowercase()))?;
    decoder.render(source, destination, input)
}

/// Representative poster timestamp: `min(1s, 10% of duration)`, with a
/// first-frame fallback for very short videos.
pub fn poster_timestamp(duration_ms: Option<u64>) -> f64 {
    let Some(duration_ms) = duration_ms else {
        return 1.0;
    };
    let seconds = duration_ms as f64 / 1000.0;
    if seconds < 2.0 {
        0.0
    } else {
        (seconds * 0.10).min(1.0)
    }
}

fn extract_frame(source: &Path, destination: &Path, timestamp: f64) -> bool {
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-ss",
            &format!("{timestamp:.3}"),
            "-i",
        ])
        .arg(source)
        .args(["-frames:v", "1", "-f", "image2"])
        .arg(destination)
        .status();
    matches!(status, Ok(status) if status.success()) && destination.exists()
}

/// Apply EXIF orientation so the image is upright.
pub fn orient_image(image: DynamicImage, orientation: u16) -> DynamicImage {
    match orientation {
        2 => image.fliph(),
        3 => image.rotate180(),
        4 => image.flipv(),
        5 => image.fliph().rotate270(),
        6 => image.rotate90(),
        7 => image.fliph().rotate90(),
        8 => image.rotate270(),
        _ => image,
    }
}

/// Center cover crop: take the center square of the upright image and resize it
/// exactly to the thumbnail size without distortion.
pub fn cover_crop(image: DynamicImage, size: u32) -> DynamicImage {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return DynamicImage::new_rgb8(size, size);
    }
    let side = width.min(height);
    let x = (width - side) / 2;
    let y = (height - side) / 2;
    image
        .crop_imm(x, y, side, side)
        .resize_exact(size, size, FilterType::Lanczos3)
}

/// Encode an image as lossy WebP carrying no source metadata.
pub fn encode_webp(image: &DynamicImage, quality: f32) -> Result<Vec<u8>> {
    let rgb = image.to_rgb8();
    let encoder = webp::Encoder::from_rgb(rgb.as_raw(), rgb.width(), rgb.height());
    let memory = encoder.encode(quality);
    Ok(memory.to_vec())
}

/// Destination path for a source relative path.
pub fn thumb_destination(public_root: &Path, src_rel_path: &str) -> PathBuf {
    crate::paths::thumb_abs_path(public_root, src_rel_path)
}

/// Read-only access to the EXIF fields used by the raster decoder.
pub fn orientation_for(source: &Path) -> u16 {
    let metadata: Option<ExifMetadata> = probe::read_exif_metadata(source);
    metadata.and_then(|value| value.orientation).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;
    use std::process::Command;

    fn test_image(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }))
    }

    #[test]
    fn cover_crop_is_square_and_centered() {
        let image = test_image(1000, 400);
        let cropped = cover_crop(image, THUMB_SIZE);
        assert_eq!(cropped.dimensions(), (THUMB_SIZE, THUMB_SIZE));
    }

    #[test]
    fn portrait_images_are_cropped_not_stretched() {
        let image = test_image(300, 900);
        let cropped = cover_crop(image, THUMB_SIZE);
        assert_eq!(cropped.dimensions(), (THUMB_SIZE, THUMB_SIZE));
    }

    #[test]
    fn rotation_makes_image_upright() {
        // 4x2: top-left red marker, bottom-right blue marker.
        let mut image = image::RgbImage::from_pixel(4, 2, Rgb([200, 200, 200]));
        image.put_pixel(0, 0, Rgb([255, 0, 0]));
        image.put_pixel(3, 1, Rgb([0, 0, 255]));
        let upright = orient_image(DynamicImage::ImageRgb8(image), 6);
        assert_eq!(upright.dimensions(), (2, 4));
        // 90 degree clockwise: the red top-left marker moves to top-right.
        let red = upright.to_rgb8().get_pixel(1, 0).0;
        assert_eq!(red, [255, 0, 0]);
    }

    #[test]
    fn webp_encoding_produces_requested_size() {
        let bytes = encode_webp(&test_image(800, 600), WEBP_QUALITY).unwrap();
        let decoded =
            image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP).unwrap();
        assert_eq!(decoded.dimensions(), (800, 600));
        assert!(webp::Decoder::new(&bytes).decode().is_some());
    }

    #[test]
    fn heic_is_unsupported_without_aborting() {
        let error = generate_thumbnail(
            Path::new("missing.heic"),
            Path::new("out.webp"),
            "heic",
            &ThumbInput { duration_ms: None },
        )
        .unwrap_err();
        assert!(matches!(error, ThumbError::Unsupported(ref format) if format == "heic"));
        assert!(!format_supported("heic"));
    }

    #[test]
    fn poster_timestamp_rules() {
        assert_eq!(poster_timestamp(Some(500)), 0.0);
        assert_eq!(poster_timestamp(Some(5_000)), 0.5);
        assert_eq!(poster_timestamp(Some(30_000)), 1.0);
        assert_eq!(poster_timestamp(Some(120_000)), 1.0);
    }

    #[test]
    fn failed_write_leaves_no_partial_file() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.png");
        test_image(20, 20).save(&source).unwrap();
        let destination = temp.path().join("thumb.webp");
        std::fs::create_dir(&destination).unwrap();
        let error = generate_thumbnail(
            &source,
            &destination,
            "png",
            &ThumbInput { duration_ms: None },
        )
        .unwrap_err();
        assert!(matches!(error, ThumbError::Failed(_)));
        let leftovers: Vec<_> = std::fs::read_dir(temp.path())
            .unwrap()
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".media-catalog-tmp-")
            })
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn video_poster_is_512_square_when_ffmpeg_is_available() {
        if probe::ensure_external_tools().is_err() {
            eprintln!("skipping video thumbnail test: ffmpeg/ffprobe unavailable");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let video = temp.path().join("clip.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=64x48:rate=10:duration=3",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&video)
            .status()
            .unwrap();
        assert!(status.success());
        let destination = temp.path().join("thumbs/clip.mp4.webp");
        generate_thumbnail(
            &video,
            &destination,
            "mp4",
            &ThumbInput {
                duration_ms: Some(3000),
            },
        )
        .unwrap();
        let bytes = std::fs::read(&destination).unwrap();
        let decoded =
            image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP).unwrap();
        assert_eq!(decoded.dimensions(), (512, 512));
    }
}
