//! The keycap artwork in `assets/icon.png`, decoded for the window and packed
//! into a Windows icon for the executable.

use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;

pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|err| format!("Could not read the icon ({err})."))?;
    let mut buf = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| "The icon is too large to decode.".to_string())?
    ];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|err| format!("Could not decode the icon ({err})."))?;
    let pixels = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => {
            let mut expanded = Vec::with_capacity(pixels.len() / 3 * 4);
            for chunk in pixels.chunks_exact(3) {
                expanded.extend_from_slice(chunk);
                expanded.push(255);
            }
            expanded
        }
        other => {
            return Err(format!(
                "The icon uses an unsupported color type ({other:?})."
            ))
        }
    };
    let expected = (info.width as usize)
        .checked_mul(info.height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| "The icon is too large to decode.".to_string())?;
    if rgba.len() != expected {
        return Err("The icon decoded to an unexpected size.".into());
    }
    Ok((info.width, info.height, rgba))
}

/// Writes a PNG-inside-ICO file. Windows Vista and later use this for the
/// file icon in Explorer and for a pinned taskbar button.
///
/// The application calls this from `build.rs`, which is a separate crate, so
/// the binary itself does not reference it.
#[cfg_attr(not(test), allow(dead_code))]
pub fn write_ico(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), String> {
    let mut sizes = vec![16u32, 24, 32, 48, 64, 96, 128, 256];
    if !sizes.contains(&width) && width > 0 && width < 256 && width == height {
        sizes.push(width);
    }
    sizes.sort_unstable();
    sizes.dedup();

    let mut images = Vec::with_capacity(sizes.len());
    for size in sizes {
        let scaled = resize(rgba, width, height, size, size);
        images.push((size, encode_png(&scaled, size, size)?));
    }

    let mut file = BufWriter::new(File::create(path).map_err(|err| err.to_string())?);
    file.write_all(&0u16.to_le_bytes())
        .map_err(|err| err.to_string())?;
    file.write_all(&1u16.to_le_bytes())
        .map_err(|err| err.to_string())?;
    file.write_all(&(images.len() as u16).to_le_bytes())
        .map_err(|err| err.to_string())?;

    let mut offset = 6 + images.len() * 16;
    for (size, png) in &images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        file.write_all(&[dim, dim, 0, 0])
            .map_err(|err| err.to_string())?;
        file.write_all(&1u16.to_le_bytes())
            .map_err(|err| err.to_string())?;
        file.write_all(&32u16.to_le_bytes())
            .map_err(|err| err.to_string())?;
        file.write_all(&(png.len() as u32).to_le_bytes())
            .map_err(|err| err.to_string())?;
        file.write_all(&(offset as u32).to_le_bytes())
            .map_err(|err| err.to_string())?;
        offset += png.len();
    }
    for (_, png) in &images {
        file.write_all(png).map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[cfg_attr(not(test), allow(dead_code))]
fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|err| format!("Could not encode an icon size ({err})."))?;
        writer
            .write_image_data(rgba)
            .map_err(|err| format!("Could not encode an icon size ({err})."))?;
    }
    Ok(out)
}

#[cfg_attr(not(test), allow(dead_code))]
fn resize(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    if sw == dw && sh == dh {
        return src.to_vec();
    }
    if dw < sw || dh < sh {
        resize_area(src, sw, sh, dw, dh)
    } else {
        resize_bilinear(src, sw, sh, dw, dh)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn resize_area(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let y0 = y * sh / dh;
        let y1 = ((y + 1) * sh / dh).max(y0 + 1).min(sh);
        for x in 0..dw {
            let x0 = x * sw / dw;
            let x1 = ((x + 1) * sw / dw).max(x0 + 1).min(sw);
            let mut rgb = [0u32; 3];
            let mut alpha = 0u32;
            let mut count = 0u32;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let pixel = at(src, sw, sx, sy);
                    let a = pixel[3] as u32;
                    for channel in 0..3 {
                        rgb[channel] += pixel[channel] as u32 * a;
                    }
                    alpha += a;
                    count += 1;
                }
            }
            let dest = ((y * dw + x) * 4) as usize;
            if alpha == 0 {
                out[dest + 3] = 0;
            } else {
                for channel in 0..3 {
                    out[dest + channel] = (rgb[channel] / alpha) as u8;
                }
                out[dest + 3] = (alpha / count) as u8;
            }
        }
    }
    out
}

#[cfg_attr(not(test), allow(dead_code))]
fn resize_bilinear(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let fy = (y as f32 + 0.5) * sh as f32 / dh as f32 - 0.5;
        let y0 = fy.floor().max(0.0) as u32;
        let y1 = (y0 + 1).min(sh - 1);
        let ty = (fy - y0 as f32).clamp(0.0, 1.0);
        for x in 0..dw {
            let fx = (x as f32 + 0.5) * sw as f32 / dw as f32 - 0.5;
            let x0 = fx.floor().max(0.0) as u32;
            let x1 = (x0 + 1).min(sw - 1);
            let tx = (fx - x0 as f32).clamp(0.0, 1.0);
            let sample = blend(
                blend(at(src, sw, x0, y0), at(src, sw, x1, y0), tx),
                blend(at(src, sw, x0, y1), at(src, sw, x1, y1), tx),
                ty,
            );
            let dest = ((y * dw + x) * 4) as usize;
            out[dest..dest + 4].copy_from_slice(&sample);
        }
    }
    out
}

#[cfg_attr(not(test), allow(dead_code))]
fn blend(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let mut out = [0u8; 4];
    for channel in 0..4 {
        let mixed = a[channel] as f32 + (b[channel] as f32 - a[channel] as f32) * t;
        out[channel] = mixed.round().clamp(0.0, 255.0) as u8;
    }
    out
}

#[cfg_attr(not(test), allow(dead_code))]
fn at(src: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [src[i], src[i + 1], src[i + 2], src[i + 3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_png_keeps_its_transparent_corners() {
        let (width, height, rgba) = decode_png(include_bytes!("../assets/icon.png")).unwrap();
        assert_eq!((width, height), (118, 118));
        assert_eq!(&rgba[..4], &[0, 0, 0, 0]);
        assert!(rgba.chunks(4).any(|pixel| pixel[3] == 255));
    }

    #[test]
    fn ico_packs_a_png_for_each_size() {
        let (width, height, rgba) = decode_png(include_bytes!("../assets/icon.png")).unwrap();
        let path = std::env::temp_dir().join("callsign-icon-test.ico");
        write_ico(&path, &rgba, width, height).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
        let count = u16::from_le_bytes([bytes[4], bytes[5]]);
        assert!(count >= 8);
        assert!(bytes
            .windows(8)
            .any(|window| window == b"\x89PNG\r\n\x1a\n"));
    }
}
