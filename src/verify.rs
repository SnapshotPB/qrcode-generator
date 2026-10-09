//! Decode check with `rqrr`, so the page can report if the symbol is readable.

use crate::render::Dot;

/// Result of the decode check.
pub struct Verification {
    pub ok: bool,
    pub decoded: Option<String>,
}

/// Rasterize the dots as a greyscale image and try to decode it.
/// Each dot is drawn with its own luminance so that light logo colors
/// count as light modules, as a camera would see them.
pub fn verify(width: usize, dots: &[Option<Dot>], bg_lum: f64, expected: &str) -> Verification {
    const SCALE: usize = 4;
    const QUIET: usize = 4;
    let size = (width + 2 * QUIET) * SCALE;
    let bg = (bg_lum.clamp(0.0, 1.0) * 255.0) as u8;
    let mut img = vec![bg; size * size];
    for r in 0..width {
        for c in 0..width {
            if let Some(dot) = dots[r * width + c] {
                let v = (dot.luminance().clamp(0.0, 1.0) * 255.0) as u8;
                let y0 = (r + QUIET) * SCALE;
                let x0 = (c + QUIET) * SCALE;
                for y in y0..y0 + SCALE {
                    for x in x0..x0 + SCALE {
                        img[y * size + x] = v;
                    }
                }
            }
        }
    }
    let mut prepared =
        rqrr::PreparedImage::prepare_from_greyscale(size, size, |x, y| img[y * size + x]);
    let grids = prepared.detect_grids();
    for grid in grids {
        if let Ok((_meta, content)) = grid.decode() {
            let ok = content == expected;
            return Verification {
                ok,
                decoded: Some(content),
            };
        }
    }
    Verification {
        ok: false,
        decoded: None,
    }
}
