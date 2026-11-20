use std::{
    path::Path,
    sync::Arc,
};

use gpui::RenderImage;
use image::{imageops::FilterType, ImageFormat};
use smallvec::smallvec;

pub fn circular_avatar_png(path: &Path) -> Result<Vec<u8>, String> {
    let source = image::open(path).map_err(|error| format!("Could not load profile image: {error}"))?;
    let (width, height) = (source.width(), source.height());
    let edge = width.min(height);
    let x = (width - edge) / 2;
    let y = (height - edge) / 2;

    let mut image = source
        .crop_imm(x, y, edge, edge)
        .resize_exact(256, 256, FilterType::Lanczos3)
        .to_rgba8();

    let center = 128.0_f32;
    let radius = 128.0_f32;
    for (px, py, pixel) in image.enumerate_pixels_mut() {
        let dx = px as f32 + 0.5 - center;
        let dy = py as f32 + 0.5 - center;
        let distance = (dx * dx + dy * dy).sqrt();
        let coverage = ((radius - distance) / 1.25).clamp(0.0, 1.0);
        pixel[3] = (pixel[3] as f32 * coverage).round() as u8;
    }

    let mut bytes = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|error| format!("Could not prepare profile image: {error}"))?;
    Ok(bytes)
}

pub fn render_image(bytes: &[u8]) -> Option<Arc<RenderImage>> {
    let decoded = image::load_from_memory(bytes).ok()?.into_rgba8();
    let mut buffer = decoded;

    // GPUI stores images in BGRA order.
    for pixel in buffer.pixels_mut() {
        pixel.0.swap(0, 2);
    }

    let frame = image::Frame::new(buffer);
    Some(Arc::new(RenderImage::new(smallvec![frame])))
}
