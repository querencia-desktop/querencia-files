// SPDX-License-Identifier: MIT

use std::io::{Cursor, Write};

use image::{ImageFormat, RgbImage};

use super::{render, render_bytes};
use crate::sandbox::{ParseOperation, png_dimensions};

fn gradient(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    })
}

fn encoded(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    gradient(width, height)
        .write_to(&mut bytes, format)
        .expect("encode fixture");
    bytes.into_inner()
}

fn dimensions(png: &[u8]) -> (u32, u32) {
    png_dimensions(png).expect("PNG output")
}

/// CRC-32 (IEEE) as PNG chunks carry it.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

#[test]
fn jpeg_and_png_scale_down_to_the_requested_edge_and_keep_small_images() {
    for format in [ImageFormat::Jpeg, ImageFormat::Png] {
        let wide = render_bytes(&encoded(300, 100, format), 64).expect("wide image");
        assert_eq!(dimensions(&wide), (64, 21), "{format:?}");
        assert!(crate::sandbox::valid_output(
            ParseOperation::ThumbnailImage,
            &wide
        ));

        let tall = render_bytes(&encoded(100, 300, format), 64).expect("tall image");
        assert_eq!(dimensions(&tall), (21, 64), "{format:?}");

        let small = render_bytes(&encoded(50, 80, format), 256).expect("small image");
        assert_eq!(dimensions(&small), (50, 80), "{format:?}: never upscaled");
        assert!(crate::sandbox::valid_output(
            ParseOperation::PreviewImage,
            &small
        ));
    }
}

#[test]
fn output_is_png_whatever_the_input_format() {
    for format in [
        ImageFormat::Jpeg,
        ImageFormat::Png,
        ImageFormat::Gif,
        ImageFormat::WebP,
        ImageFormat::Bmp,
    ] {
        let png = render_bytes(&encoded(40, 30, format), 32).expect("decoded");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "{format:?}");
        assert_eq!(dimensions(&png), (32, 24), "{format:?}");
    }
}

#[test]
fn damaged_and_unknown_inputs_fail_without_panicking() {
    let jpeg = encoded(120, 90, ImageFormat::Jpeg);
    let truncated = &jpeg[..jpeg.len() / 3];
    assert!(render_bytes(truncated, 64).is_err());

    let mut png = encoded(120, 90, ImageFormat::Png);
    let middle = png.len() / 2;
    for byte in &mut png[middle..middle + 16] {
        *byte ^= 0xff;
    }
    assert!(render_bytes(&png, 64).is_err());

    let error = render_bytes(b"this is not an image at all", 64).expect_err("text");
    assert!(error.contains("unrecognised"), "{error}");

    assert!(render_bytes(b"", 64).is_err());
}

#[test]
fn formats_the_fallback_does_not_handle_are_refused_by_name() {
    let error =
        render_bytes(b"P3\n2 2\n255\n0 0 0 0 0 0 0 0 0 0 0 0\n", 64).expect_err("portable pixmap");
    assert!(
        error.contains("not handled") || error.contains("unrecognised"),
        "{error}"
    );
}

#[test]
fn oversized_dimensions_are_rejected_before_decoding() {
    // A well-formed PNG header claiming 100 000 × 100 000 pixels with no pixel
    // data behind it: the decoder must refuse on the dimensions, not allocate.
    let mut chunk = Vec::new();
    chunk.extend_from_slice(b"IHDR");
    chunk.extend_from_slice(&100_000u32.to_be_bytes());
    chunk.extend_from_slice(&100_000u32.to_be_bytes());
    chunk.extend_from_slice(&[8, 6, 0, 0, 0]);
    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(&chunk);
    png.extend_from_slice(&crc32(&chunk).to_be_bytes());
    // Empty IDAT and IEND so the header parses completely; still no pixels.
    for chunk in [b"IDAT", b"IEND"] {
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(chunk);
        png.extend_from_slice(&crc32(chunk).to_be_bytes());
    }
    let error = render_bytes(&png, 64).expect_err("oversized header");
    assert!(
        error.contains("limit") || error.contains("budget") || error.contains("dimension"),
        "{error}"
    );
}

#[test]
fn files_are_read_from_the_given_path_only() {
    let mut file = tempfile::NamedTempFile::new().expect("fixture");
    file.write_all(&encoded(64, 64, ImageFormat::Png))
        .expect("write fixture");
    let png = render(file.path(), 16).expect("decoded from path");
    assert_eq!(dimensions(&png), (16, 16));

    let mut text = tempfile::NamedTempFile::new().expect("text fixture");
    text.write_all(b"plain text").expect("write text");
    assert!(render(text.path(), 16).is_err());
    assert!(render(std::path::Path::new("/nonexistent/image.png"), 16).is_err());
}
