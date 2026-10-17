//! assetc: converts PNG art (e.g. from PixelLab) into GBA-style 4bpp tiles.
//!
//!     assetc <in.png> <out-stem> [--colors N] [--cell WxH] [--scale N]
//!
//! Transparent pixels (alpha < 128) become color 0, which is transparent on
//! every GBA layer; the rest are quantized to at most `--colors` (1–15,
//! default 15), ordered light to dark as indices 1..=N. `--scale N` first
//! shrinks art that was exported N× upscaled. `--cell WxH` orders tiles per
//! W×H cell, row-major inside each cell: the GBA's 1D sprite mapping, so a
//! sheet of 16×32 frames converts with `--cell 16x32`. The default 8x8 keeps
//! plain row-major tile order.
//!
//! Writes `<out-stem>.4bpp`, `<out-stem>.pal` (16 × little-endian RGB555,
//! one palette bank) and `<out-stem>.preview.png` showing the quantized result.

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::process::ExitCode;

type Rgb = [f32; 3];

struct Image {
    w: usize,
    h: usize,
    /// RGBA8.
    px: Vec<[u8; 4]>,
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("assetc: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    const USAGE: &str = "usage: assetc <in.png> <out-stem> [--colors N] [--cell WxH] [--scale N]";
    let mut positional = Vec::new();
    let (mut colors, mut cell, mut scale) = (15usize, (8usize, 8usize), 1usize);
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or(USAGE);
        match arg.as_str() {
            "--colors" => colors = value()?.parse().map_err(|_| USAGE)?,
            "--scale" => scale = value()?.parse().map_err(|_| USAGE)?,
            "--cell" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or(USAGE)?;
                cell = (w.parse().map_err(|_| USAGE)?, h.parse().map_err(|_| USAGE)?);
            }
            _ => positional.push(arg),
        }
    }
    let [input, stem] = positional.as_slice() else { return Err(USAGE.into()) };
    if !(1..=15).contains(&colors) || scale == 0 || cell.0 == 0 || cell.1 == 0 || cell.0 % 8 != 0 || cell.1 % 8 != 0 {
        return Err(USAGE.into());
    }

    let img = downscale(read_png(Path::new(input))?, scale);
    if img.w % cell.0 != 0 || img.h % cell.1 != 0 {
        return Err(format!("image is {}×{}, not a whole number of {}×{} cells", img.w, img.h, cell.0, cell.1));
    }

    let opaque = |p: &[u8; 4]| p[3] >= 128;
    let pixels: Vec<Rgb> = img.px.iter().filter(|p| opaque(p)).map(to_rgb).collect();
    let centroids = quantize(&pixels, colors);

    let mut palette = [0u16; 16];
    for (i, c) in centroids.iter().enumerate() {
        palette[i + 1] = rgb555(*c);
    }
    let indices: Vec<u8> = img
        .px
        .iter()
        .map(|p| if opaque(p) { (nearest(&centroids, to_rgb(p)) + 1) as u8 } else { 0 })
        .collect();

    let tiles = encode_tiles(&indices, img.w, img.h, cell);
    std::fs::write(format!("{stem}.4bpp"), &tiles).map_err(|e| e.to_string())?;
    let pal: Vec<u8> = palette.iter().flat_map(|c| c.to_le_bytes()).collect();
    std::fs::write(format!("{stem}.pal"), pal).map_err(|e| e.to_string())?;
    write_preview(&format!("{stem}.preview.png"), &img, &indices, &palette)?;

    println!("{input}: {}×{} → {} tiles, {} colors", img.w, img.h, tiles.len() / 32, centroids.len());
    Ok(())
}

fn read_png(path: &Path) -> Result<Image, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let bytes = &buf[..info.buffer_size()];
    let px = match info.color_type {
        png::ColorType::Rgba => bytes.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect(),
        png::ColorType::Rgb => bytes.chunks_exact(3).map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => bytes.chunks_exact(2).map(|c| [c[0], c[0], c[0], c[1]]).collect(),
        png::ColorType::Grayscale => bytes.iter().map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".into()),
    };
    Ok(Image { w: info.width as usize, h: info.height as usize, px })
}

/// Nearest-neighbour shrink, sampling the centre of each N×N block.
fn downscale(img: Image, n: usize) -> Image {
    if n == 1 {
        return img;
    }
    let (w, h) = (img.w / n, img.h / n);
    let px = (0..w * h)
        .map(|i| img.px[((i / w) * n + n / 2) * img.w + (i % w) * n + n / 2])
        .collect();
    Image { w, h, px }
}

fn to_rgb(p: &[u8; 4]) -> Rgb {
    [p[0] as f32, p[1] as f32, p[2] as f32]
}

fn luma(c: &Rgb) -> f32 {
    0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
}

fn dist(a: &Rgb, b: &Rgb) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum()
}

fn nearest(centroids: &[Rgb], c: Rgb) -> usize {
    (0..centroids.len())
        .min_by(|&a, &b| dist(&centroids[a], &c).total_cmp(&dist(&centroids[b], &c)))
        .unwrap()
}

/// k-means seeded at luminance quantiles; returns `k` colors, lightest first.
fn quantize(colors: &[Rgb], k: usize) -> Vec<Rgb> {
    let mut unique: Vec<Rgb> = Vec::new();
    for c in colors {
        if !unique.contains(c) {
            unique.push(*c);
            if unique.len() > k {
                break;
            }
        }
    }
    let mut centroids = if unique.len() <= k {
        unique
    } else {
        let mut sorted = colors.to_vec();
        sorted.sort_by(|a, b| luma(a).total_cmp(&luma(b)));
        let mut centroids: Vec<Rgb> = (0..k).map(|i| sorted[(2 * i + 1) * sorted.len() / (2 * k)]).collect();
        for _ in 0..24 {
            let mut sums = vec![([0f32; 3], 0usize); k];
            for c in colors {
                let s = &mut sums[nearest(&centroids, *c)];
                (0..3).for_each(|i| s.0[i] += c[i]);
                s.1 += 1;
            }
            for (centroid, (sum, n)) in centroids.iter_mut().zip(sums) {
                if n > 0 {
                    *centroid = sum.map(|v| v / n as f32);
                }
            }
        }
        centroids
    };
    centroids.sort_by(|a, b| luma(b).total_cmp(&luma(a)));
    centroids
}

fn rgb555(c: Rgb) -> u16 {
    let ch = |v: f32| (v.round().clamp(0.0, 255.0) as u16) >> 3;
    ch(c[0]) | (ch(c[1]) << 5) | (ch(c[2]) << 10)
}

/// Emits 8×8 tiles in GBA 4bpp format (low nibble = left pixel), ordered
/// row-major within each cell and cells row-major over the image.
fn encode_tiles(indices: &[u8], w: usize, h: usize, (cw, ch): (usize, usize)) -> Vec<u8> {
    let mut out = Vec::new();
    for cy in (0..h).step_by(ch) {
        for cx in (0..w).step_by(cw) {
            for ty in (cy..cy + ch).step_by(8) {
                for tx in (cx..cx + cw).step_by(8) {
                    for y in 0..8 {
                        for x in (0..8).step_by(2) {
                            let at = (ty + y) * w + tx + x;
                            out.push(indices[at] | indices[at + 1] << 4);
                        }
                    }
                }
            }
        }
    }
    out
}

fn write_preview(path: &str, img: &Image, indices: &[u8], palette: &[u16; 16]) -> Result<(), String> {
    let expand = |v: u16| ((v << 3) | (v >> 2)) as u8;
    let data: Vec<u8> = indices
        .iter()
        .flat_map(|&i| {
            let c = palette[i as usize];
            let alpha = if i == 0 { 0 } else { 255 };
            [expand(c & 31), expand((c >> 5) & 31), expand((c >> 10) & 31), alpha]
        })
        .collect();
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), img.w as u32, img.h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_orders_light_to_dark() {
        let colors = [[0.0; 3], [255.0; 3], [128.0; 3], [64.0; 3]];
        let q = quantize(&colors, 4);
        assert_eq!(q, [[255.0; 3], [128.0; 3], [64.0; 3], [0.0; 3]]);
    }

    #[test]
    fn quantize_reduces_many_colors() {
        let colors: Vec<Rgb> = (0..=255).map(|v| [v as f32; 3]).collect();
        let q = quantize(&colors, 4);
        assert_eq!(q.len(), 4);
        assert!(q.windows(2).all(|w| luma(&w[0]) > luma(&w[1])));
    }

    #[test]
    fn encodes_like_the_game_expects() {
        let mut indices = vec![0u8; 64];
        indices[0] = 1;
        indices[1] = 15;
        let t = encode_tiles(&indices, 8, 8, (8, 8));
        assert_eq!(t.len(), 32);
        assert_eq!(t[0], 0xF1);
    }

    #[test]
    fn cells_follow_1d_sprite_mapping() {
        // A 16×16 image as one cell: the second tile is the top-right quadrant.
        let mut indices = vec![0u8; 256];
        indices[8] = 3;
        let t = encode_tiles(&indices, 16, 16, (16, 16));
        assert_eq!(t[32], 0x03);
    }
}
