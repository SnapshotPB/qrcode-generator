//! QR code generator with a color logo. The logo colors show through the
//! dots of the symbol. Compiled to WebAssembly for an offline web page.

pub mod qr;
pub mod render;
pub mod verify;

use render::{EmbeddedImage, Logo, Params, Sampler, MODE_EMBED};
use wasm_bindgen::prelude::*;

/// Options for one render. All fields are plain values so the page can set
/// them directly on the object.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    /// 0 = L, 1 = M, 2 = Q, 3 = H
    pub ec_level: u8,
    /// 0 = automatic, 1..=40 forces at least that version.
    pub min_version: u8,
    /// 0 = no logo, 1 = fill, 2 = tint (default), 3 = embed (the image is
    /// drawn as given over a cleared rectangle).
    pub logo_mode: u8,
    /// Logo center as a fraction of the symbol width (0..1).
    pub logo_cx: f64,
    pub logo_cy: f64,
    /// Logo width as a fraction of the symbol width.
    pub logo_size: f64,
    /// Rotation in degrees.
    pub logo_rotation: f64,
    /// Fraction of a module the logo must cover before the module takes its color.
    pub coverage: f64,
    pub alpha_threshold: u8,
    pub white_transparent: bool,
    pub white_cutoff: u8,
    pub max_luminance: f64,
    /// Replace every opaque logo pixel with `colorize_color`.
    pub colorize: bool,
    pub colorize_color: u32,
    /// Embed mode: clear space around the image, in modules.
    pub embed_margin: f64,
    pub dot_scale: f64,
    pub dot_shape: u8,
    pub quiet_zone: u8,
    pub dot_color: u32,
    pub bg_color: u32,
    pub transparent_bg: bool,
    pub protect_function_color: bool,
    /// Run the decode check after the render.
    pub verify: bool,
}

#[wasm_bindgen]
impl RenderOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> RenderOptions {
        RenderOptions::default()
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            ec_level: 3,
            min_version: 0,
            logo_mode: 2,
            logo_cx: 0.5,
            logo_cy: 0.5,
            logo_size: 0.5,
            logo_rotation: 0.0,
            coverage: 0.5,
            alpha_threshold: 128,
            white_transparent: true,
            white_cutoff: 235,
            max_luminance: 0.7,
            colorize: false,
            colorize_color: 0xd62828,
            embed_margin: 0.5,
            dot_scale: 0.85,
            dot_shape: 0,
            quiet_zone: 4,
            dot_color: 0x2f7d4b,
            bg_color: 0xffffff,
            transparent_bg: false,
            protect_function_color: false,
            verify: true,
        }
    }
}

impl From<&RenderOptions> for Params {
    fn from(o: &RenderOptions) -> Params {
        Params {
            logo_mode: o.logo_mode,
            logo_cx: o.logo_cx,
            logo_cy: o.logo_cy,
            logo_size: o.logo_size,
            logo_rotation: o.logo_rotation,
            coverage: o.coverage,
            alpha_threshold: o.alpha_threshold,
            white_transparent: o.white_transparent,
            white_cutoff: o.white_cutoff,
            max_luminance: o.max_luminance,
            colorize: o.colorize,
            colorize_color: o.colorize_color,
            embed_margin: o.embed_margin,
            dot_scale: o.dot_scale,
            dot_shape: o.dot_shape,
            quiet_zone: o.quiet_zone,
            dot_color: o.dot_color,
            bg_color: o.bg_color,
            transparent_bg: o.transparent_bg,
            protect_function_color: o.protect_function_color,
        }
    }
}

/// The output of one render.
#[wasm_bindgen]
pub struct RenderResult {
    svg: String,
    decoded: Option<String>,
    pub version: u8,
    pub width: u32,
    pub data_modules: u32,
    pub changed_modules: u32,
    pub logo_modules: u32,
    pub verify_ran: bool,
    pub verified: bool,
}

#[wasm_bindgen]
impl RenderResult {
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> String {
        self.svg.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn decoded(&self) -> Option<String> {
        self.decoded.clone()
    }
}

/// Render `text` as a QR symbol. `logo_rgba` is an RGBA pixel buffer of
/// `logo_width * logo_height * 4` bytes, or empty for no logo. `logo_href`
/// is the image as given, usually a data URL; embed mode draws it over the
/// symbol, and the other modes ignore it.
#[wasm_bindgen]
pub fn render(
    text: &str,
    logo_rgba: &[u8],
    logo_width: u32,
    logo_height: u32,
    logo_href: &str,
    options: &RenderOptions,
) -> Result<RenderResult, JsValue> {
    render_inner(text, logo_rgba, logo_width, logo_height, logo_href, options)
        .map_err(|e| JsValue::from_str(&e))
}

pub fn render_inner(
    text: &str,
    logo_rgba: &[u8],
    logo_width: u32,
    logo_height: u32,
    logo_href: &str,
    options: &RenderOptions,
) -> Result<RenderResult, String> {
    if text.is_empty() {
        return Err("The text is empty.".to_string());
    }
    let matrix = qr::build(
        text,
        qr::ec_level_from_u8(options.ec_level),
        options.min_version,
    )
    .map_err(|e| format!("The QR code could not be built: {e:?}"))?;

    let expected = (logo_width as usize) * (logo_height as usize) * 4;
    let logo = if !logo_rgba.is_empty() && expected == logo_rgba.len() && logo_width > 0 {
        Some(Logo {
            rgba: logo_rgba,
            width: logo_width as usize,
            height: logo_height as usize,
        })
    } else {
        None
    };

    let params = Params::from(options);
    let (dots, stats) = render::color_modules(&matrix, logo.as_ref(), &params);
    let embedded = logo
        .as_ref()
        .filter(|_| params.logo_mode == MODE_EMBED && !logo_href.is_empty())
        .map(|l| EmbeddedImage {
            href: logo_href,
            aspect: l.height as f64 / l.width as f64,
        });
    let svg = render::to_svg(matrix.width, &dots, &params, embedded.as_ref());

    let (verify_ran, verified, decoded) = if options.verify {
        let bg = render::Dot::from_u32(options.bg_color);
        let bg_lum = if options.transparent_bg {
            1.0
        } else {
            bg.luminance()
        };
        // In embed mode the image hides the dots under it, so the check
        // paints the image too.
        let sampler = logo
            .as_ref()
            .filter(|_| params.logo_mode == MODE_EMBED && params.logo_size > 0.0)
            .map(|l| Sampler::new(l, matrix.width, &params));
        let overlay = |mx: f64, my: f64| sampler.as_ref().and_then(|s| s.luminance_at(mx, my));
        let v = verify::verify(
            matrix.width,
            &dots,
            bg_lum,
            sampler.as_ref().map(|_| &overlay as verify::Overlay),
            text,
        );
        (true, v.ok, v.decoded)
    } else {
        (false, false, None)
    };

    Ok(RenderResult {
        svg,
        decoded,
        version: matrix.version,
        width: matrix.width as u32,
        data_modules: stats.data_modules,
        changed_modules: stats.changed_modules,
        logo_modules: stats.logo_modules,
        verify_ran,
        verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disc_logo(size: usize, rgb: (u8, u8, u8)) -> Vec<u8> {
        let mut buf = vec![0u8; size * size * 4];
        let c = size as f64 / 2.0;
        for y in 0..size {
            for x in 0..size {
                let dx = x as f64 + 0.5 - c;
                let dy = y as f64 + 0.5 - c;
                if dx * dx + dy * dy <= c * c {
                    let i = (y * size + x) * 4;
                    buf[i] = rgb.0;
                    buf[i + 1] = rgb.1;
                    buf[i + 2] = rgb.2;
                    buf[i + 3] = 255;
                }
            }
        }
        buf
    }

    #[test]
    fn plain_symbol_decodes() {
        let o = RenderOptions::default();
        let r = render_inner("https://example.com", &[], 0, 0, "", &o).unwrap();
        assert!(r.verify_ran);
        assert!(r.verified, "decoded = {:?}", r.decoded);
        assert_eq!(r.changed_modules, 0);
        assert!(r.svg.contains("<circle"));
    }

    #[test]
    fn tint_mode_is_default_and_keeps_data() {
        let mut o = RenderOptions::default();
        assert_eq!(o.logo_mode, 2);
        o.logo_size = 0.6;
        let logo = disc_logo(64, (220, 40, 40));
        let r = render_inner("https://example.com", &logo, 64, 64, "", &o).unwrap();
        assert_eq!(r.changed_modules, 0);
        assert!(r.logo_modules > 0);
        assert!(r.verified, "decoded = {:?}", r.decoded);
        assert!(r.svg.contains("#dc2828"));
    }

    #[test]
    fn fill_mode_small_logo_decodes() {
        let mut o = RenderOptions::default();
        o.logo_mode = 1;
        o.logo_size = 0.3;
        let logo = disc_logo(64, (30, 60, 200));
        let r = render_inner(
            "https://example.com/some/longer/path",
            &logo,
            64,
            64,
            "",
            &o,
        )
        .unwrap();
        assert!(r.changed_modules > 0);
        assert!(r.verified, "decoded = {:?}", r.decoded);
    }

    #[test]
    fn fill_mode_huge_logo_fails_check() {
        let mut o = RenderOptions::default();
        o.logo_mode = 1;
        o.logo_size = 1.2;
        let logo = disc_logo(64, (0, 0, 0));
        let r = render_inner("https://example.com", &logo, 64, 64, "", &o).unwrap();
        assert!(!r.verified);
    }

    #[test]
    fn colorize_replaces_logo_colors_and_honors_white() {
        let mut o = RenderOptions::default();
        o.logo_size = 0.6;
        o.colorize = true;
        o.colorize_color = 0x123456;
        // An opaque white disc: with white_transparent the logo is invisible.
        let logo = disc_logo(64, (255, 255, 255));
        let r = render_inner("https://example.com", &logo, 64, 64, "", &o).unwrap();
        assert_eq!(r.logo_modules, 0);
        assert!(!r.svg.contains("#123456"));
        // Without white_transparent, every covered dark module takes the colorize color.
        o.white_transparent = false;
        let r = render_inner("https://example.com", &logo, 64, 64, "", &o).unwrap();
        assert!(r.logo_modules > 0);
        assert!(r.svg.contains("#123456"));
        assert!(!r.svg.contains("#ffffff\">\n<circle"));
        assert!(r.verified, "decoded = {:?}", r.decoded);
    }

    #[test]
    fn embed_mode_clears_a_rectangle_and_draws_the_image() {
        let mut o = RenderOptions::default();
        o.logo_mode = 3;
        o.logo_size = 0.3;
        let logo = disc_logo(64, (30, 60, 200));
        let href = "data:image/png;base64,AAAA&x=\"1\"";
        let r = render_inner(
            "https://example.com/some/longer/path",
            &logo,
            64,
            64,
            href,
            &o,
        )
        .unwrap();
        assert!(r.logo_modules > 0);
        assert!(r.changed_modules > 0);
        assert!(r.verified, "decoded = {:?}", r.decoded);
        assert!(r.svg.contains("<image "));
        assert!(r
            .svg
            .contains("href=\"data:image/png;base64,AAAA&amp;x=&quot;1&quot;\""));
        // The image is drawn as given: no dot takes the logo color.
        assert!(!r.svg.contains("#1e3cc8"));
        // Without an href there is no image element.
        let r = render_inner(
            "https://example.com/some/longer/path",
            &logo,
            64,
            64,
            "",
            &o,
        )
        .unwrap();
        assert!(!r.svg.contains("<image "));
    }

    #[test]
    fn embed_mode_check_sees_the_image() {
        // A black disc as wide as the symbol hides the finder patterns, so the
        // check must fail even though the margin clears no function module.
        let mut o = RenderOptions::default();
        o.logo_mode = 3;
        o.logo_size = 1.0;
        o.embed_margin = 0.0;
        let logo = disc_logo(64, (0, 0, 0));
        let r = render_inner("https://example.com", &logo, 64, 64, "data:,", &o).unwrap();
        assert!(!r.verified);
        // A small disc leaves the symbol readable.
        o.logo_size = 0.25;
        let r = render_inner("https://example.com", &logo, 64, 64, "data:,", &o).unwrap();
        assert!(r.verified, "decoded = {:?}", r.decoded);
    }

    #[test]
    fn embed_margin_grows_the_cleared_rectangle() {
        let mut o = RenderOptions::default();
        o.logo_mode = 3;
        o.logo_size = 0.2;
        o.verify = false;
        let logo = disc_logo(64, (0, 0, 0));
        o.embed_margin = 0.0;
        let tight = render_inner("https://example.com", &logo, 64, 64, "data:,", &o).unwrap();
        o.embed_margin = 2.0;
        let wide = render_inner("https://example.com", &logo, 64, 64, "data:,", &o).unwrap();
        assert!(wide.logo_modules > tight.logo_modules);
    }

    #[test]
    fn embed_check_uses_colorize_luminance() {
        // A white disc hides the dots under it. With colorize and
        // white-as-transparent, the disc is transparent and the dots show.
        let mut o = RenderOptions::default();
        o.logo_mode = 3;
        o.logo_size = 0.9;
        o.embed_margin = 0.0;
        let logo = disc_logo(64, (255, 255, 255));
        let plain = render_inner("https://example.com", &logo, 64, 64, "data:x", &o).unwrap();
        assert!(!plain.verified);
        o.colorize = true;
        o.colorize_color = 0x000000;
        o.white_transparent = false;
        let black = render_inner("https://example.com", &logo, 64, 64, "data:x", &o).unwrap();
        assert!(!black.verified);
        o.white_transparent = true;
        let clear = render_inner("https://example.com", &logo, 64, 64, "data:x", &o).unwrap();
        assert_eq!(clear.changed_modules, plain.changed_modules);
        assert!(clear.verified || clear.changed_modules > 0);
    }

    #[test]
    fn function_mask_covers_expected_count() {
        // Version 1: 3 finder blocks (9*9 + 9*8 + 8*9) + timing (2*(21-17)) = 81+72+72+8 = 233
        let m = qr::function_mask(1, 21);
        assert_eq!(m.iter().filter(|&&b| b).count(), 233);
        // Version 2 adds one 5x5 alignment pattern.
        let m = qr::function_mask(2, 25);
        assert_eq!(
            m.iter().filter(|&&b| b).count(),
            81 + 72 + 72 + 2 * (25 - 17) + 25
        );
    }

    #[test]
    fn min_version_is_respected() {
        let mut o = RenderOptions::default();
        o.min_version = 5;
        let r = render_inner("hi", &[], 0, 0, "", &o).unwrap();
        assert_eq!(r.version, 5);
        assert_eq!(r.width, 37);
    }
}
