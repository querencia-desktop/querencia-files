// SPDX-License-Identifier: MIT

//! Pure-Rust raster decoding through the `image` crate, used inside the
//! decoder sandbox when gdk-pixbuf cannot decode there (its loaders run
//! through glycin's own sandbox on current distributions) and no external
//! renderer is installed. It reads the same already-authorised input as the
//! other decoders and keeps their frame budget and output limits.

use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Seek},
    path::Path,
};

use image::{
    DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};

#[cfg(test)]
mod tests;

/// Formats this decoder handles; others stay with the remaining renderers.
const FORMATS: [ImageFormat; 5] = [
    ImageFormat::Jpeg,
    ImageFormat::Png,
    ImageFormat::Gif,
    ImageFormat::WebP,
    ImageFormat::Bmp,
];

/// Decoded pixel budget: the same 512 MiB the frame budget allows, enforced by
/// the decoders that track allocations (PNG, JPEG) and by the dimension check
/// for the rest.
const MAX_ALLOC_BYTES: u64 = crate::sandbox::FILE_SIZE_LIMIT_BYTES;
const MAX_EDGE: u32 = crate::services::image_conversion::MAX_EDGE;

/// Renders the image at `path` as a PNG no larger than `size` on its longer
/// edge; smaller images keep their native size.
pub(super) fn render(path: &Path, size: i32) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("image fallback: {error}"))?;
    render_reader(BufReader::new(file), size)
}

pub(super) fn render_bytes(data: &[u8], size: i32) -> Result<Vec<u8>, String> {
    render_reader(Cursor::new(data), size)
}

fn render_reader<R: BufRead + Seek>(reader: R, size: i32) -> Result<Vec<u8>, String> {
    let reader = ImageReader::new(reader)
        .with_guessed_format()
        .map_err(|error| format!("image fallback: {error}"))?;
    let format = reader
        .format()
        .ok_or_else(|| "image fallback: unrecognised image format".to_owned())?;
    if !FORMATS.contains(&format) {
        return Err(format!("image fallback: {format:?} is not handled"));
    }
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| format!("image fallback: {error}"))?;
    // Dimensions are checked before any pixel allocation; allocation tracking
    // follows for decoders that support it.
    decoder
        .set_limits(limits())
        .map_err(|error| format!("image fallback: {error}"))?;
    let (width, height) = decoder.dimensions();
    let (budget_width, budget_height) = (
        i32::try_from(width).unwrap_or(i32::MAX),
        i32::try_from(height).unwrap_or(i32::MAX),
    );
    if width == 0 || height == 0 || super::exceeds_decoded_frame_budget(budget_width, budget_height)
    {
        return Err("Image dimensions exceed the decoded frame budget".to_owned());
    }
    let image =
        DynamicImage::from_decoder(decoder).map_err(|error| format!("image fallback: {error}"))?;
    let target = u32::try_from(size.max(1)).unwrap_or(1);
    let image = if width.max(height) > target {
        image.thumbnail(target, target)
    } else {
        image
    };
    let image = match image {
        DynamicImage::ImageLuma8(_)
        | DynamicImage::ImageLumaA8(_)
        | DynamicImage::ImageRgb8(_)
        | DynamicImage::ImageRgba8(_) => image,
        other => DynamicImage::ImageRgba8(other.to_rgba8()),
    };
    let mut png = Vec::new();
    image
        .write_with_encoder(PngEncoder::new_with_quality(
            &mut png,
            CompressionType::Fast,
            FilterType::NoFilter,
        ))
        .map_err(|error| format!("image fallback: {error}"))?;
    if png.len() as u64 > crate::sandbox::MAX_OUTPUT_BYTES {
        return Err("image fallback: output exceeds the size limit".to_owned());
    }
    Ok(png)
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_EDGE);
    limits.max_image_height = Some(MAX_EDGE);
    limits.max_alloc = Some(MAX_ALLOC_BYTES);
    limits
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
