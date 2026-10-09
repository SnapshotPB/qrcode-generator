//! Logo sampling, module coloring and SVG output.

use crate::qr::Matrix;

pub const MODE_OFF: u8 = 0;
pub const MODE_FILL: u8 = 1;
pub const MODE_TINT: u8 = 2;
pub const MODE_EMBED: u8 = 3;

/// Options that control the logo placement and the drawing style.
/// All lengths are fractions of the symbol width (without quiet zone),
/// so the same values work for every QR version.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// 0 = no logo, 1 = fill (logo shape overrides modules), 2 = tint (only
    /// dark modules take the logo color), 3 = embed (the image is drawn as
    /// given over a cleared rectangle).
    pub logo_mode: u8,
    pub logo_cx: f64,
    pub logo_cy: f64,
    pub logo_size: f64,
    pub logo_rotation: f64,
    /// Fraction of the module area that the logo must cover (0..1).
    pub coverage: f64,
    pub alpha_threshold: u8,
    pub white_transparent: bool,
    pub white_cutoff: u8,
    /// Logo colors brighter than this luminance (0..1) are darkened.
    pub max_luminance: f64,
    /// Replace every opaque logo pixel with `colorize_color`.
    pub colorize: bool,
    pub colorize_color: u32,
    /// Embed mode: clear space around the image, in modules.
    pub embed_margin: f64,
    /// Dot diameter relative to the module size (0..1].
    pub dot_scale: f64,
    /// 0 = circle, 1 = square, 2 = rounded square.
    pub dot_shape: u8,
    pub quiet_zone: u8,
    pub dot_color: u32,
    pub bg_color: u32,
    pub transparent_bg: bool,
    /// Dark modules of function patterns keep `dot_color` if `true`.
    pub protect_function_color: bool,
}

/// A drawn module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Dot {
    pub fn from_u32(c: u32) -> Self {
        Dot {
            r: ((c >> 16) & 0xff) as u8,
            g: ((c >> 8) & 0xff) as u8,
            b: (c & 0xff) as u8,
        }
    }

    pub fn luminance(&self) -> f64 {
        luminance(self.r, self.g, self.b)
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

pub fn luminance(r: u8, g: u8, b: u8) -> f64 {
    (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64) / 255.0
}

/// RGBA pixel buffer of the logo.
pub struct Logo<'a> {
    pub rgba: &'a [u8],
    pub width: usize,
    pub height: usize,
}

/// Statistics of a render, reported to the page.
#[derive(Default, Debug)]
pub struct Stats {
    pub data_modules: u32,
    pub changed_modules: u32,
    pub logo_modules: u32,
    pub dark_modules: u32,
}

/// Decide the color of every module. `None` means the module is light.
pub fn color_modules(m: &Matrix, logo: Option<&Logo>, p: &Params) -> (Vec<Option<Dot>>, Stats) {
    let w = m.width;
    let base = Dot::from_u32(p.dot_color);
    let mut dots: Vec<Option<Dot>> = Vec::with_capacity(w * w);
    let mut stats = Stats::default();

    let sampler = logo
        .filter(|_| p.logo_mode != MODE_OFF && p.logo_size > 0.0)
        .map(|l| Sampler::new(l, w, p));

    for r in 0..w {
        for c in 0..w {
            let i = m.idx(r, c);
            let dark = m.dark[i];
            let function = m.function[i];
            if !function {
                stats.data_modules += 1;
            }

            let dot = if p.logo_mode == MODE_EMBED {
                let under_image = sampler.as_ref().is_some_and(|s| s.in_footprint(r, c));
                if under_image && !function {
                    // Embed mode: the rectangle under the image holds no dots.
                    if dark {
                        stats.changed_modules += 1;
                    }
                    stats.logo_modules += 1;
                    None
                } else if dark {
                    Some(base)
                } else {
                    None
                }
            } else {
                let logo_color = sampler.as_ref().and_then(|s| s.sample(r, c));
                match (logo_color, p.logo_mode) {
                    (Some(color), MODE_FILL) => {
                        // Fill mode: the logo shape overrides data modules.
                        if function {
                            if dark {
                                Some(if p.protect_function_color {
                                    base
                                } else {
                                    color
                                })
                            } else {
                                None
                            }
                        } else {
                            if !dark {
                                stats.changed_modules += 1;
                            }
                            stats.logo_modules += 1;
                            Some(color)
                        }
                    }
                    (Some(color), MODE_TINT) => {
                        // Tint mode: only dark modules take the logo color.
                        if dark {
                            stats.logo_modules += 1;
                            if function && p.protect_function_color {
                                Some(base)
                            } else {
                                Some(color)
                            }
                        } else {
                            None
                        }
                    }
                    _ => {
                        if dark {
                            Some(base)
                        } else {
                            None
                        }
                    }
                }
            };
            if dot.is_some() {
                stats.dark_modules += 1;
            }
            dots.push(dot);
        }
    }
    (dots, stats)
}

/// Maps module positions to logo pixels and averages the covered color.
pub struct Sampler<'a> {
    logo: &'a Logo<'a>,
    cx: f64,
    cy: f64,
    /// Width of the logo in modules.
    size_modules: f64,
    scale: f64, // logo pixels per module
    cos: f64,
    sin: f64,
    p: Params,
}

const SUPER: usize = 4;

impl<'a> Sampler<'a> {
    pub fn new(logo: &'a Logo<'a>, width: usize, p: &Params) -> Self {
        let w = width as f64;
        let size_modules = (p.logo_size * w).max(1e-6);
        let angle = p.logo_rotation.to_radians();
        Sampler {
            logo,
            cx: p.logo_cx * w,
            cy: p.logo_cy * w,
            size_modules,
            scale: logo.width as f64 / size_modules,
            cos: angle.cos(),
            sin: angle.sin(),
            p: *p,
        }
    }

    /// Height of the logo in modules.
    fn height_modules(&self) -> f64 {
        self.size_modules * self.logo.height as f64 / self.logo.width as f64
    }

    /// Rotate a point in symbol coordinates (modules) into the logo frame,
    /// with the origin at the logo center.
    fn to_logo_frame(&self, mx: f64, my: f64) -> (f64, f64) {
        let mx = mx - self.cx;
        let my = my - self.cy;
        (
            mx * self.cos + my * self.sin,
            -mx * self.sin + my * self.cos,
        )
    }

    /// The logo pixel under a point in symbol coordinates (modules).
    fn pixel_at(&self, mx: f64, my: f64) -> Option<(u8, u8, u8, u8)> {
        let (lw, lh) = (self.logo.width as f64, self.logo.height as f64);
        let (rx, ry) = self.to_logo_frame(mx, my);
        let px = rx * self.scale + lw / 2.0;
        let py = ry * self.scale + lh / 2.0;
        if px < 0.0 || py < 0.0 || px >= lw || py >= lh {
            return None;
        }
        let idx = ((py as usize) * self.logo.width + px as usize) * 4;
        Some((
            self.logo.rgba[idx],
            self.logo.rgba[idx + 1],
            self.logo.rgba[idx + 2],
            self.logo.rgba[idx + 3],
        ))
    }

    /// Luminance of the opaque logo pixel under a point in symbol
    /// coordinates (modules), as a scanner sees the embedded image.
    /// With colorize, the page draws every opaque pixel in the colorize
    /// color and makes white pixels transparent when that option is on,
    /// so the check applies the same rule.
    pub fn luminance_at(&self, mx: f64, my: f64) -> Option<f64> {
        let (r, g, b, a) = self.pixel_at(mx, my)?;
        if a < self.p.alpha_threshold {
            return None;
        }
        if self.p.colorize {
            if self.p.white_transparent
                && r >= self.p.white_cutoff
                && g >= self.p.white_cutoff
                && b >= self.p.white_cutoff
            {
                return None;
            }
            return Some(Dot::from_u32(self.p.colorize_color).luminance());
        }
        Some(luminance(r, g, b))
    }

    /// `true` when any part of the module lies in the rectangle of the
    /// image plus the embed margin.
    pub fn in_footprint(&self, row: usize, col: usize) -> bool {
        let margin = self.p.embed_margin.max(0.0);
        let half_w = self.size_modules / 2.0 + margin;
        let half_h = self.height_modules() / 2.0 + margin;
        for j in 0..SUPER {
            for i in 0..SUPER {
                let mx = col as f64 + (i as f64 + 0.5) / SUPER as f64;
                let my = row as f64 + (j as f64 + 0.5) / SUPER as f64;
                let (rx, ry) = self.to_logo_frame(mx, my);
                if rx.abs() <= half_w && ry.abs() <= half_h {
                    return true;
                }
            }
        }
        false
    }

    /// Returns the average logo color under the module, or `None` when the
    /// logo does not cover enough of the module.
    fn sample(&self, row: usize, col: usize) -> Option<Dot> {
        let mut hit = 0usize;
        let (mut sr, mut sg, mut sb) = (0u32, 0u32, 0u32);
        for j in 0..SUPER {
            for i in 0..SUPER {
                let mx = col as f64 + (i as f64 + 0.5) / SUPER as f64;
                let my = row as f64 + (j as f64 + 0.5) / SUPER as f64;
                let Some((r, g, b, a)) = self.pixel_at(mx, my) else {
                    continue;
                };
                if a < self.p.alpha_threshold {
                    continue;
                }
                if self.p.white_transparent
                    && r >= self.p.white_cutoff
                    && g >= self.p.white_cutoff
                    && b >= self.p.white_cutoff
                {
                    continue;
                }
                hit += 1;
                sr += r as u32;
                sg += g as u32;
                sb += b as u32;
            }
        }
        let total = SUPER * SUPER;
        if hit == 0 || (hit as f64) / (total as f64) < self.p.coverage.clamp(0.0, 1.0) {
            return None;
        }
        let n = hit as u32;
        let mut dot = if self.p.colorize {
            Dot::from_u32(self.p.colorize_color)
        } else {
            Dot {
                r: (sr / n) as u8,
                g: (sg / n) as u8,
                b: (sb / n) as u8,
            }
        };
        let lum = dot.luminance();
        if lum > self.p.max_luminance && lum > 0.0 {
            let k = self.p.max_luminance / lum;
            dot = Dot {
                r: (dot.r as f64 * k) as u8,
                g: (dot.g as f64 * k) as u8,
                b: (dot.b as f64 * k) as u8,
            };
        }
        Some(dot)
    }
}

/// The image that embed mode draws over the symbol.
pub struct EmbeddedImage<'a> {
    /// The `href` of the SVG `<image>`, usually a data URL.
    pub href: &'a str,
    /// Height of the image divided by its width.
    pub aspect: f64,
}

fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Produce the SVG document. `image` is drawn over the dots in embed mode.
pub fn to_svg(
    width: usize,
    dots: &[Option<Dot>],
    p: &Params,
    image: Option<&EmbeddedImage>,
) -> String {
    let q = p.quiet_zone as usize;
    let total = width + 2 * q;
    let d = p.dot_scale.clamp(0.05, 1.0);
    let r = d / 2.0;
    // At full size adjacent squares share an edge and anti-aliasing leaves a
    // faint seam. A small overlap removes it.
    let bleed = if p.dot_shape != 0 && d >= 0.999 {
        0.03
    } else {
        0.0
    };
    let side = d + 2.0 * bleed;
    let mut svg = String::with_capacity(width * width * 40);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {t} {t}\" width=\"{t}\" height=\"{t}\" shape-rendering=\"geometricPrecision\">\n",
        t = total
    ));
    if !p.transparent_bg {
        svg.push_str(&format!(
            "<rect width=\"{t}\" height=\"{t}\" fill=\"{bg}\"/>\n",
            t = total,
            bg = Dot::from_u32(p.bg_color).hex()
        ));
    }
    // Group dots by color so repeated fills stay small.
    let mut current: Option<Dot> = None;
    let mut order: Vec<(Dot, Vec<(usize, usize)>)> = Vec::new();
    for row in 0..width {
        for col in 0..width {
            if let Some(dot) = dots[row * width + col] {
                if current != Some(dot) {
                    current = Some(dot);
                    match order.iter_mut().find(|(c, _)| *c == dot) {
                        Some(entry) => entry.1.push((row, col)),
                        None => order.push((dot, vec![(row, col)])),
                    }
                } else if let Some(entry) = order.iter_mut().find(|(c, _)| *c == dot) {
                    entry.1.push((row, col));
                }
            }
        }
    }
    for (dot, cells) in order {
        svg.push_str(&format!("<g fill=\"{}\">\n", dot.hex()));
        for (row, col) in cells {
            let x = (col + q) as f64;
            let y = (row + q) as f64;
            match p.dot_shape {
                1 => svg.push_str(&format!(
                    "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{side:.3}\" height=\"{side:.3}\"/>\n",
                    x + 0.5 - r - bleed,
                    y + 0.5 - r - bleed
                )),
                2 => svg.push_str(&format!(
                    "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{side:.3}\" height=\"{side:.3}\" rx=\"{:.3}\"/>\n",
                    x + 0.5 - r - bleed,
                    y + 0.5 - r - bleed,
                    r * 0.5
                )),
                _ => svg.push_str(&format!(
                    "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{r:.3}\"/>\n",
                    x + 0.5,
                    y + 0.5
                )),
            }
        }
        svg.push_str("</g>\n");
    }
    if let Some(image) = image.filter(|_| p.logo_mode == MODE_EMBED && p.logo_size > 0.0) {
        let w = width as f64;
        let iw = p.logo_size * w;
        let ih = iw * image.aspect;
        let cx = q as f64 + p.logo_cx * w;
        let cy = q as f64 + p.logo_cy * w;
        svg.push_str(&format!(
            "<image x=\"{x:.3}\" y=\"{y:.3}\" width=\"{iw:.3}\" height=\"{ih:.3}\" preserveAspectRatio=\"none\" transform=\"rotate({rot:.2} {cx:.3} {cy:.3})\" href=\"{href}\"/>\n",
            x = cx - iw / 2.0,
            y = cy - ih / 2.0,
            rot = p.logo_rotation,
            href = escape_attr(image.href),
        ));
    }
    svg.push_str("</svg>\n");
    svg
}
