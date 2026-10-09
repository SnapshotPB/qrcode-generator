//! QR matrix construction and the mask of function patterns.

use qrcode::{Color, EcLevel, QrCode, QrResult, Version};

/// A QR symbol as a square matrix of dark/light modules.
pub struct Matrix {
    pub version: u8,
    pub width: usize,
    /// `true` for a dark module. Row-major, `width * width` entries.
    pub dark: Vec<bool>,
    /// `true` for a module that is part of a function pattern
    /// (finder, separator, timing, alignment, format or version info).
    /// These modules must keep their value or the symbol is not readable.
    pub function: Vec<bool>,
}

impl Matrix {
    #[inline]
    pub fn idx(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }
}

pub fn ec_level_from_u8(level: u8) -> EcLevel {
    match level {
        0 => EcLevel::L,
        1 => EcLevel::M,
        2 => EcLevel::Q,
        _ => EcLevel::H,
    }
}

/// Build the matrix for `text`. `min_version` is 0 for automatic selection,
/// or 1..=40 to force at least that symbol version.
pub fn build(text: &str, ec: EcLevel, min_version: u8) -> QrResult<Matrix> {
    let code = if min_version == 0 {
        QrCode::with_error_correction_level(text.as_bytes(), ec)?
    } else {
        let mut result = None;
        for v in min_version.clamp(1, 40)..=40 {
            match QrCode::with_version(text.as_bytes(), Version::Normal(v as i16), ec) {
                Ok(code) => {
                    result = Some(Ok(code));
                    break;
                }
                Err(e) => result = Some(Err(e)),
            }
        }
        result.unwrap()?
    };

    let version = match code.version() {
        Version::Normal(v) => v as u8,
        Version::Micro(_) => 1,
    };
    let width = code.width();
    let dark: Vec<bool> = code
        .to_colors()
        .into_iter()
        .map(|c| c == Color::Dark)
        .collect();
    let function = function_mask(version, width);
    Ok(Matrix {
        version,
        width,
        dark,
        function,
    })
}

/// Center coordinates of the alignment patterns for each version (1..=40).
const ALIGNMENT: [&[usize]; 41] = [
    &[],
    &[],
    &[6, 18],
    &[6, 22],
    &[6, 26],
    &[6, 30],
    &[6, 34],
    &[6, 22, 38],
    &[6, 24, 42],
    &[6, 26, 46],
    &[6, 28, 50],
    &[6, 30, 54],
    &[6, 32, 58],
    &[6, 34, 62],
    &[6, 26, 46, 66],
    &[6, 26, 48, 70],
    &[6, 26, 50, 74],
    &[6, 30, 54, 78],
    &[6, 30, 56, 82],
    &[6, 30, 58, 86],
    &[6, 34, 62, 90],
    &[6, 28, 50, 72, 94],
    &[6, 26, 50, 74, 98],
    &[6, 30, 54, 78, 102],
    &[6, 28, 54, 80, 106],
    &[6, 32, 58, 84, 110],
    &[6, 30, 58, 86, 114],
    &[6, 34, 62, 90, 118],
    &[6, 26, 50, 74, 98, 122],
    &[6, 30, 54, 78, 102, 126],
    &[6, 26, 52, 78, 104, 130],
    &[6, 30, 56, 82, 108, 134],
    &[6, 34, 60, 86, 112, 138],
    &[6, 30, 58, 86, 114, 142],
    &[6, 34, 62, 90, 118, 146],
    &[6, 30, 54, 78, 102, 126, 150],
    &[6, 24, 50, 76, 102, 128, 154],
    &[6, 28, 54, 80, 106, 132, 158],
    &[6, 32, 58, 84, 110, 136, 162],
    &[6, 26, 54, 82, 110, 138, 166],
    &[6, 30, 58, 86, 114, 142, 170],
];

/// Mark every module that belongs to a function pattern.
pub fn function_mask(version: u8, width: usize) -> Vec<bool> {
    let mut mask = vec![false; width * width];
    let mut fill = |r0: usize, r1: usize, c0: usize, c1: usize| {
        for r in r0..r1.min(width) {
            for c in c0..c1.min(width) {
                mask[r * width + c] = true;
            }
        }
    };

    // Finder patterns, separators and format information.
    fill(0, 9, 0, 9);
    fill(0, 9, width - 8, width);
    fill(width - 8, width, 0, 9);

    // Timing patterns.
    fill(6, 7, 0, width);
    fill(0, width, 6, 7);

    // Version information.
    if version >= 7 {
        fill(0, 6, width - 11, width - 8);
        fill(width - 11, width - 8, 0, 6);
    }

    // Alignment patterns.
    let centers = ALIGNMENT[version as usize];
    if let (Some(&first), Some(&last)) = (centers.first(), centers.last()) {
        for &cy in centers {
            for &cx in centers {
                let in_finder = (cy == first && cx == first)
                    || (cy == first && cx == last)
                    || (cy == last && cx == first);
                if in_finder {
                    continue;
                }
                fill(cy - 2, cy + 3, cx - 2, cx + 3);
            }
        }
    }
    mask
}
