// SPDX-License-Identifier: MIT

//! Querencia: the default theme "Querencia Klassisch" follows the GTK theme.
//! While the GTK theme is light its catalog palette applies unchanged (it
//! repeats Querencia-Klassisch's colours); once the GTK theme turns dark
//! (colour scheme "dark" with a theme that has a dark variant) the palette is
//! taken from the theme's named colours, so Files darkens with the desktop
//! and stays light under a theme without a dark variant.

use gtk::prelude::*;

use super::{ThemeTokens, blend, color_to_hex, omarchy::contrast, parse_rgb_channels};

pub(super) const SYSTEM_THEME_ID: &str = "querencia-classic";

const TEXT_CONTRAST: f64 = 4.5;

/// The GTK theme's named colours the palette is built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ThemeColors {
    pub base: String,
    pub background: String,
    pub foreground: String,
    pub accent: String,
    pub borders: String,
    pub error: String,
}

/// Reads the named colours of the GTK theme currently loaded for the
/// default display.
pub(super) fn current_theme_colors() -> Option<ThemeColors> {
    gtk::gdk::Display::default()?;
    let probe = gtk::Label::new(None);
    #[expect(
        deprecated,
        reason = "GTK exposes named theme colors through StyleContext"
    )]
    let context = probe.style_context();
    let lookup = |names: &[&str]| {
        names.iter().find_map(|name| {
            #[expect(
                deprecated,
                reason = "GTK exposes named theme colors through StyleContext"
            )]
            let color = context.lookup_color(name);
            color.map(|color| color_to_hex(&color.to_str()))
        })
    };
    Some(ThemeColors {
        base: lookup(&["theme_base_color", "view_bg_color"])?,
        background: lookup(&["theme_bg_color", "window_bg_color"])?,
        foreground: lookup(&["theme_fg_color", "window_fg_color"])?,
        accent: lookup(&["accent_color", "theme_selected_bg_color", "accent_bg_color"])?,
        borders: lookup(&["borders"])?,
        error: lookup(&["error_color"]).unwrap_or_else(|| "#cc0000".to_owned()),
    })
}

fn is_dark(color: &str) -> bool {
    parse_rgb_channels(color).is_some_and(|[red, green, blue]| {
        u32::from(red) * 299 + u32::from(green) * 587 + u32::from(blue) * 114 < 128_000
    })
}

/// The first blend of `color` towards `pole` that reaches `ratio` against
/// every background, or `pole` itself.
fn readable(color: &str, backgrounds: &[&str], pole: &str, ratio: f64) -> String {
    (0..=20)
        .map(|step| blend(color, pole, f64::from(step) / 20.0))
        .find(|candidate| {
            backgrounds
                .iter()
                .all(|background| contrast(candidate, background) >= ratio)
        })
        .unwrap_or_else(|| color_to_hex(pole))
}

/// The palette for a dark GTK theme, or `None` while the theme is light and
/// the catalog palette `light` stays in effect.
pub(super) fn dark_tokens(light: &ThemeTokens, colors: &ThemeColors) -> Option<ThemeTokens> {
    if !is_dark(&colors.background) {
        return None;
    }
    let background = colors.base.clone();
    let surface = colors.background.clone();
    let text = colors.foreground.clone();
    let muted = blend(&surface, &text, 0.08);
    let highlight = blend(&surface, &text, 0.16);
    let backgrounds = [background.as_str(), surface.as_str(), muted.as_str()];
    let dim_text = readable(
        &blend(&text, &surface, 0.35),
        &backgrounds,
        &text,
        TEXT_CONTRAST,
    );
    let danger = readable(&colors.error, &backgrounds[..2], &text, TEXT_CONTRAST);
    let syntax = |color: &Option<String>| {
        color
            .as_deref()
            .map(|color| readable(color, &backgrounds[..2], &text, TEXT_CONTRAST))
    };
    Some(ThemeTokens {
        name: light.name.clone(),
        syntax_keyword: syntax(&light.syntax_keyword),
        syntax_string: syntax(&light.syntax_string),
        syntax_constant: syntax(&light.syntax_constant),
        syntax_type: syntax(&light.syntax_type),
        syntax_preprocessor: syntax(&light.syntax_preprocessor),
        accent: colors.accent.clone(),
        border: colors.borders.clone(),
        background,
        surface,
        text,
        danger,
        muted,
        highlight,
        dim_text,
    })
}

#[cfg(test)]
mod tests;
