//! Design tokens: a TOML file of two palettes, fonts, sizes, a chart series, radii and spacing,
//! written as CSS variables and taught to the utility generator as colour names.
//!
//! The vocabulary is swatchbook's: `[light]` and `[dark]` with the same nine colours
//! ([`NAMES`]), `[fonts]` and `[sizes]`; on top, `[series]` (a list per scheme), `[radii]` and
//! `[space]` (CSS lengths by name). [`Tokens::css`] writes the light palette on `:root`, the
//! dark one under `prefers-color-scheme: dark` unless `[data-theme=light]` says otherwise, and
//! both again under `[data-theme=...]`, so a page (or its host) can pick a scheme for the
//! whole page or for one subtree.
//!
//! A [`Theme`] changes single tokens of a file before the variables are written, so a change
//! holds in every scheme and under `[data-theme]`; [`Tokens::pairs`] are the contrast duties
//! to check it against.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// The nine colour names of a palette, in swatchbook's order.
pub const NAMES: [&str; 9] = [
    "ground", "ink", "muted", "accent", "surface", "border", "ok", "warn", "stop",
];

/// A token file, as read.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    /// Dark text on a light ground.
    pub light: Palette,
    /// Light text on a dark ground.
    pub dark: Palette,
    /// The font families.
    pub fonts: Fonts,
    /// Text sizes: points in a diagram, CSS pixels on a page.
    #[serde(default)]
    pub sizes: Sizes,
    /// The categorical series of a chart, per scheme.
    #[serde(default)]
    pub series: Series,
    /// Corner radii by name: `--radius-<name>`.
    #[serde(default)]
    pub radii: BTreeMap<String, String>,
    /// Spacing by name: `--space-<name>`.
    #[serde(default)]
    pub space: BTreeMap<String, String>,
}

/// One scheme's colours, each `#rrggbb`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    missing_docs,
    reason = "the fields are NAMES, documented in the token file"
)]
pub struct Palette {
    pub ground: String,
    pub ink: String,
    pub muted: String,
    pub accent: String,
    pub surface: String,
    pub border: String,
    pub ok: String,
    pub warn: String,
    pub stop: String,
}

impl Palette {
    /// Every colour with its name, in [`NAMES`] order.
    #[must_use]
    pub fn named(&self) -> [(&'static str, &str); 9] {
        let values = [
            &self.ground,
            &self.ink,
            &self.muted,
            &self.accent,
            &self.surface,
            &self.border,
            &self.ok,
            &self.warn,
            &self.stop,
        ];
        let mut out = [("", ""); 9];
        for (slot, (name, value)) in out.iter_mut().zip(NAMES.iter().zip(values)) {
            *slot = (name, value.as_str());
        }
        out
    }

    /// The colour called `name`, one of [`NAMES`].
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.named()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v)
    }

    fn slot(&mut self, name: &str) -> Option<&mut String> {
        Some(match name {
            "ground" => &mut self.ground,
            "ink" => &mut self.ink,
            "muted" => &mut self.muted,
            "accent" => &mut self.accent,
            "surface" => &mut self.surface,
            "border" => &mut self.border,
            "ok" => &mut self.ok,
            "warn" => &mut self.warn,
            "stop" => &mut self.stop,
            _ => return None,
        })
    }
}

/// Changes to a token file: `[theme.light]`, `[theme.dark]` and `[theme.fonts]`, each key
/// optional, an unknown one refused.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    /// Colours of the light palette.
    pub light: PaletteOverlay,
    /// Colours of the dark palette.
    pub dark: PaletteOverlay,
    /// Font families.
    pub fonts: FontsOverlay,
}

/// Some colours of one palette, each `#rrggbb`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[allow(
    missing_docs,
    reason = "the fields are NAMES, documented in the token file"
)]
pub struct PaletteOverlay {
    pub ground: Option<String>,
    pub ink: Option<String>,
    pub muted: Option<String>,
    pub accent: Option<String>,
    pub surface: Option<String>,
    pub border: Option<String>,
    pub ok: Option<String>,
    pub warn: Option<String>,
    pub stop: Option<String>,
}

impl PaletteOverlay {
    /// The colours given, with their names, in [`NAMES`] order.
    #[must_use]
    pub fn named(&self) -> Vec<(&'static str, &str)> {
        let values = [
            &self.ground,
            &self.ink,
            &self.muted,
            &self.accent,
            &self.surface,
            &self.border,
            &self.ok,
            &self.warn,
            &self.stop,
        ];
        NAMES
            .iter()
            .zip(values)
            .filter_map(|(name, value)| value.as_deref().map(|v| (*name, v)))
            .collect()
    }
}

/// Some font families.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FontsOverlay {
    /// Proportional text.
    pub sans: Option<String>,
    /// Code and figures.
    pub mono: Option<String>,
    /// Headings.
    pub display: Option<String>,
}

/// The font families, by name; [`Tokens::css`] appends the system fallbacks.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fonts {
    /// Proportional text.
    pub sans: String,
    /// Code and figures.
    pub mono: String,
    /// Headings.
    pub display: String,
}

/// Text sizes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sizes {
    /// Body text.
    pub text: f32,
    /// Labels and captions.
    pub label: f32,
}

impl Default for Sizes {
    fn default() -> Self {
        Self {
            text: 13.0,
            label: 11.0,
        }
    }
}

/// A chart's series colours, slot 1 first.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Series {
    /// On the light ground.
    pub light: Vec<String>,
    /// On the dark ground.
    pub dark: Vec<String>,
}

impl Tokens {
    /// Read and check a token file.
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(&text).with_context(|| path.display().to_string())
    }

    /// Parse and check a token file's text: every colour `#rrggbb`, both series the same length.
    pub fn parse(text: &str) -> Result<Self> {
        let tokens: Self = toml::from_str(text)?;
        for (scheme, palette) in [("light", &tokens.light), ("dark", &tokens.dark)] {
            for (name, value) in palette.named() {
                rgb(value).with_context(|| format!("[{scheme}] {name}"))?;
            }
        }
        for (scheme, list) in [
            ("light", &tokens.series.light),
            ("dark", &tokens.series.dark),
        ] {
            for value in list {
                rgb(value).with_context(|| format!("[series] {scheme}"))?;
            }
        }
        if tokens.series.light.len() != tokens.series.dark.len() {
            bail!("[series]: light and dark have a different number of colours");
        }
        Ok(tokens)
    }

    /// These tokens with `theme`'s changes; a colour that is not `#rrggbb` is an error.
    pub fn with(mut self, theme: &Theme) -> Result<Self> {
        for (scheme, palette, over) in [
            ("light", &mut self.light, &theme.light),
            ("dark", &mut self.dark, &theme.dark),
        ] {
            for (name, value) in over.named() {
                rgb(value).with_context(|| format!("[theme.{scheme}] {name}"))?;
                if let Some(slot) = palette.slot(name) {
                    value.clone_into(slot);
                }
            }
        }
        let fonts = &theme.fonts;
        for (slot, value) in [
            (&mut self.fonts.sans, &fonts.sans),
            (&mut self.fonts.mono, &fonts.mono),
            (&mut self.fonts.display, &fonts.display),
        ] {
            if let Some(value) = value {
                value.clone_into(slot);
            }
        }
        Ok(self)
    }

    /// Every pair that must be readable, in both schemes, with its ratio: `ink`, `muted`,
    /// `accent`, `ok`, `warn` and `stop` at 4.5:1 (WCAG AA for text) on `ground` and on
    /// `surface`, each series colour at 3:1 (a graphical object) on both, and `ground` at
    /// 4.5:1 on `accent` (a filled button's label).
    #[must_use]
    pub fn pairs(&self) -> Vec<Pair> {
        let mut out = vec![];
        for (scheme, palette, series) in [
            ("light", &self.light, &self.series.light),
            ("dark", &self.dark, &self.series.dark),
        ] {
            let mut push = |fg: String, fg_hex: &str, bg: &'static str, bg_hex: &str, need| {
                let ratio = match (rgb(fg_hex), rgb(bg_hex)) {
                    (Ok(a), Ok(b)) => contrast(a, b),
                    _ => 1.0,
                };
                out.push(Pair {
                    scheme,
                    fg,
                    fg_hex: fg_hex.to_owned(),
                    bg,
                    bg_hex: bg_hex.to_owned(),
                    ratio,
                    need,
                });
            };
            for back in ["ground", "surface"] {
                let bg = palette.get(back).unwrap_or_default();
                for name in TEXT {
                    let fg = palette.get(name).unwrap_or_default();
                    push(name.to_owned(), fg, back, bg, 4.5);
                }
                for (i, fg) in series.iter().enumerate() {
                    push(format!("series-{}", i + 1), fg, back, bg, 3.0);
                }
            }
            push(
                "ground".into(),
                &palette.ground,
                "accent",
                &palette.accent,
                4.5,
            );
        }
        out
    }

    /// The colour names a utility may use, each to its variable: `ground` to `var(--ground)`,
    /// `series-1` to `var(--series-1)`.
    #[must_use]
    pub fn colors(&self) -> Vec<(String, String)> {
        let series = (1..=self.series.light.len()).map(|i| format!("series-{i}"));
        NAMES
            .iter()
            .map(|n| (*n).to_owned())
            .chain(series)
            .map(|n| {
                let var = format!("var(--{n})");
                (n, var)
            })
            .collect()
    }

    /// The variables: the scheme-independent ones and the light palette on `:root`, the dark
    /// palette for a dark system preference (unless `<html data-theme=light>`), and each
    /// palette again under its `[data-theme]` - on `<html>` for the page, or on any element
    /// for a subtree in the other scheme.
    #[must_use]
    pub fn css(&self) -> String {
        let light = scheme(&self.light, &self.series.light);
        let dark = scheme(&self.dark, &self.series.dark);
        let mut common = String::new();
        let fonts = [
            ("sans", &self.fonts.sans, SANS),
            ("mono", &self.fonts.mono, MONO),
            ("display", &self.fonts.display, SANS),
        ];
        for (name, family, fallback) in fonts {
            let _ = write!(common, "--font-{name}:\"{family}\",{fallback};");
        }
        let _ = write!(
            common,
            "--size-text:{}px;--size-label:{}px;",
            self.sizes.text, self.sizes.label
        );
        for (name, value) in &self.radii {
            let _ = write!(common, "--radius-{name}:{value};");
        }
        for (name, value) in &self.space {
            let _ = write!(common, "--space-{name}:{value};");
        }
        // The label on a filled accent: the ground, checked for contrast against it.
        common.push_str("--on-accent:var(--ground);");
        format!(
            ":root{{color-scheme:light dark;{common}{light}}}\n\
             @media (prefers-color-scheme: dark){{:root:not([data-theme=light]){{{dark}}}}}\n\
             [data-theme=light]{{color-scheme:light;{light}}}\n\
             [data-theme=dark]{{color-scheme:dark;{dark}}}\n"
        )
    }
}

/// The colours that carry text, held to 4.5:1 on `ground` and `surface`.
const TEXT: [&str; 6] = ["ink", "muted", "accent", "ok", "warn", "stop"];

/// One contrast duty of [`Tokens::pairs`]: a colour on a background, in one scheme.
#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    /// `light` or `dark`.
    pub scheme: &'static str,
    /// The foreground's name: one of [`NAMES`] or `series-<n>`.
    pub fg: String,
    /// The foreground colour.
    pub fg_hex: String,
    /// The background's name.
    pub bg: &'static str,
    /// The background colour.
    pub bg_hex: String,
    /// The WCAG 2 contrast ratio of the two.
    pub ratio: f64,
    /// The ratio the pair needs.
    pub need: f64,
}

impl Pair {
    /// The ratio is at least the one needed.
    #[must_use]
    pub fn passes(&self) -> bool {
        self.ratio >= self.need
    }

    /// The ratio for a message, rounded down to one decimal so a failure never reads as the
    /// ratio it needs: `4.47` is `4.4`.
    #[must_use]
    pub fn shown(&self) -> f64 {
        (self.ratio * 10.0).floor() / 10.0
    }
}

/// The system fallbacks after a sans family.
const SANS: &str = "ui-sans-serif,system-ui,-apple-system,\"Segoe UI\",Roboto,sans-serif";
/// The system fallbacks after a mono family.
const MONO: &str = "ui-monospace,\"SF Mono\",Menlo,Consolas,monospace";

/// One scheme's colour variables.
fn scheme(palette: &Palette, series: &[String]) -> String {
    let mut out = String::new();
    for (name, value) in palette.named() {
        let _ = write!(out, "--{name}:{value};");
    }
    for (i, value) in series.iter().enumerate() {
        let _ = write!(out, "--series-{}:{value};", i + 1);
    }
    out
}

/// `#rrggbb` as its three channels.
pub fn rgb(hex: &str) -> Result<[u8; 3]> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("`{hex}` is not a #rrggbb colour");
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16);
    Ok([channel(0)?, channel(2)?, channel(4)?])
}

/// The WCAG 2 contrast ratio of two colours, from 1 to 21.
#[must_use]
pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |c: [u8; 3]| {
        let lin = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.040_45 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.0722f64.mul_add(lin(c[2]), 0.2126f64.mul_add(lin(c[0]), 0.7152 * lin(c[1])))
    };
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panelkit() -> Tokens {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../panelkit/tokens.toml");
        Tokens::load(&path).unwrap()
    }

    /// Every pair that must be readable, with the ratio it needs; one line per failure.
    #[test]
    fn panelkit_tokens_reach_wcag_aa_on_ground_and_surface() {
        let pairs = panelkit().pairs();
        assert_eq!(pairs.len(), 2 * (2 * (6 + 6) + 1));
        let failures: Vec<_> = pairs
            .iter()
            .filter(|p| !p.passes())
            .map(|p| {
                format!(
                    "{}: {} on {} is {:.2}, needs {}",
                    p.scheme, p.fg, p.bg, p.ratio, p.need
                )
            })
            .collect();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn a_theme_changes_single_tokens_and_refuses_unknown_ones() {
        let theme: Theme = toml::from_str(
            "[light]\naccent = \"#5b4fb3\"\n[dark]\naccent = \"#b4a9f5\"\n[fonts]\nmono = \"Mono X\"\n",
        )
        .unwrap();
        let tokens = panelkit().with(&theme).unwrap();
        assert_eq!(tokens.light.accent, "#5b4fb3");
        assert_eq!(tokens.dark.accent, "#b4a9f5");
        assert_eq!(tokens.dark.ground, "#17191f", "the rest stays");
        assert_eq!(tokens.fonts.mono, "Mono X");
        assert!(toml::from_str::<Theme>("[dark]\naccnet = \"#000000\"\n").is_err());
        assert!(toml::from_str::<Theme>("[sizes]\ntext = 1\n").is_err());
        let bad: Theme = toml::from_str("[dark]\naccent = \"lavender\"\n").unwrap();
        let error = panelkit().with(&bad).unwrap_err();
        assert!(
            format!("{error:#}").starts_with("[theme.dark] accent: "),
            "{error:#}"
        );
    }

    #[test]
    fn a_shown_ratio_rounds_down() {
        let pair = Pair {
            scheme: "dark",
            fg: "accent".into(),
            fg_hex: String::new(),
            bg: "surface",
            bg_hex: String::new(),
            ratio: 4.479,
            need: 4.5,
        };
        assert!((pair.shown() - 4.4).abs() < 1e-9);
        assert!(!pair.passes());
    }

    #[test]
    fn the_ratio_matches_known_values() {
        assert!((contrast([0, 0, 0], [255, 255, 255]) - 21.0).abs() < 1e-9);
        assert!((contrast([0x77, 0x77, 0x77], [255, 255, 255]) - 4.48).abs() < 0.01);
    }

    #[test]
    fn the_css_has_both_schemes_and_the_series() {
        let css = panelkit().css();
        assert!(css.starts_with(":root{color-scheme:light dark;--font-sans:\"Inter\","));
        assert!(css.contains("--ground:#fbfaf7;"), "{css}");
        assert!(
            css.contains(":root:not([data-theme=light]){--ground:#17191f;"),
            "{css}"
        );
        assert!(css.contains("\n[data-theme=dark]{color-scheme:dark;--ground:#17191f;"));
        assert!(css.contains("--series-6:"), "{css}");
        assert!(css.contains("--radius-md:8px;") && css.contains("--space-gap:12px;"));
    }

    #[test]
    fn a_bad_colour_or_an_unknown_key_is_refused() {
        let good = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../panelkit/tokens.toml"),
        )
        .unwrap();
        assert!(Tokens::parse(&good.replace("#fbfaf7", "red")).is_err());
        assert!(Tokens::parse(&format!("{good}\n[extra]\na = 1\n")).is_err());
        assert!(Tokens::parse(&good.replace("ground = \"#17191f\"", "")).is_err());
    }
}
