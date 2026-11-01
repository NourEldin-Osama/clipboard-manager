//! Makes `assets/app-icon/app.ico` from the drawings next to it: `app-small.svg`
//! for 16 to 24 px, `app.svg` for the larger sizes. build.rs puts the .ico in
//! the exe. Run it after changing a drawing; give a folder to also get a PNG
//! of each size to look at:
//!
//!     cargo run --example app_icon -- [folder for the PNGs]

use std::path::Path;

use anyhow::Context as _;

/// The sizes Windows asks an icon for.
const SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// Up to this size the bolder drawing is used.
const SMALL_UP_TO: u32 = 24;

fn main() -> anyhow::Result<()> {
    let dir = Path::new("assets/app-icon");
    let big = std::fs::read(dir.join("app.svg"))?;
    let small = std::fs::read(dir.join("app-small.svg"))?;
    let previews = std::env::args().nth(1);
    let mut images = Vec::new();
    for size in SIZES {
        let rgba = render(if size <= SMALL_UP_TO { &small } else { &big }, size)?;
        if let Some(previews) = &previews {
            std::fs::create_dir_all(previews)?;
            let path = Path::new(previews).join(format!("app-{size}.png"));
            std::fs::write(path, png(&rgba, size)?)?;
        }
        images.push((size, rgba));
    }
    std::fs::write(dir.join("app.ico"), ico(&images)?)?;
    println!("wrote {}", dir.join("app.ico").display());
    Ok(())
}

/// The drawing at `size` by `size` pixels, as RGBA without premultiplied alpha.
fn render(svg: &[u8], size: u32) -> anyhow::Result<Vec<u8>> {
    let hint = egui::SizeHint::Size {
        width: size,
        height: size,
        maintain_aspect_ratio: true,
    };
    let image = egui_extras::image::load_svg_bytes_with_size(svg, hint, &Default::default())
        .map_err(anyhow::Error::msg)?;
    anyhow::ensure!(image.size == [size as usize; 2], "the drawing isn't square");
    Ok(image
        .pixels
        .iter()
        .flat_map(|c| c.to_srgba_unmultiplied())
        .collect())
}

fn png(rgba: &[u8], size: u32) -> anyhow::Result<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_raw(size, size, rgba.to_vec())
        .context("the pixels don't fill the image")?
        .write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

/// An .ico file: a PNG for 256 px and Windows bitmaps for the rest, which is
/// what every program that reads icons understands.
fn ico(images: &[(u32, Vec<u8>)]) -> anyhow::Result<Vec<u8>> {
    let mut entries = Vec::new();
    for (size, rgba) in images {
        entries.push(if *size >= 256 {
            png(rgba, *size)?
        } else {
            bitmap(rgba, *size)
        });
    }
    let mut out = Vec::new();
    // Reserved, type 1 (icon), number of images.
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for ((size, _), data) in images.iter().zip(&entries) {
        // Width and height, where 0 means 256; no palette; reserved.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[side, side, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for data in entries {
        out.extend_from_slice(&data);
    }
    Ok(out)
}

/// A 32-bit Windows bitmap as an icon keeps it: the header gives twice the
/// height (the colours, then a mask), rows run from the bottom up in blue,
/// green, red, alpha order, and the mask stays empty because the alpha
/// already says what is see-through.
fn bitmap(rgba: &[u8], size: u32) -> Vec<u8> {
    let side = size as usize;
    let mut out = Vec::new();
    out.extend_from_slice(&40u32.to_le_bytes()); // header size
    out.extend_from_slice(&(size as i32).to_le_bytes());
    out.extend_from_slice(&(2 * size as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
                                                 // No compression; the rest (sizes, resolution, palette) may be zero.
    out.extend_from_slice(&[0; 24]);
    for row in rgba.chunks_exact(side * 4).rev() {
        for [r, g, b, a] in row.as_chunks::<4>().0 {
            out.extend_from_slice(&[*b, *g, *r, *a]);
        }
    }
    // One bit a pixel, each row padded to four bytes.
    out.resize(out.len() + side.div_ceil(32) * 4 * side, 0);
    out
}
