// SPDX-License-Identifier: MIT

use gtk::prelude::*;

use super::{SYSTEM_THEME_ID, ThemeColors, dark_tokens};
use crate::{
    test_support::gtk_test,
    ui::theme::{ThemeManager, ThemeTokens, builtins, omarchy::contrast},
};

fn querencia_classic() -> ThemeTokens {
    builtins()
        .into_iter()
        .find(|theme| theme.id == SYSTEM_THEME_ID)
        .expect("bundled Querencia Klassisch")
        .tokens
}

/// The named colours of Querencia-Klassisch 0.2.1 under the dark colour scheme.
fn querencia_klassisch_dark() -> ThemeColors {
    ThemeColors {
        base: "#2b2a27".to_owned(),
        background: "#3c3b37".to_owned(),
        foreground: "#dfdbd2".to_owned(),
        accent: "#f07746".to_owned(),
        borders: "#23221f".to_owned(),
        error: "#cc0000".to_owned(),
    }
}

const DARK_THEME_CSS: &str = "@define-color theme_base_color #2b2a27;\n@define-color theme_bg_color #3c3b37;\n@define-color theme_fg_color #dfdbd2;\n@define-color accent_color #f07746;\n@define-color borders #23221f;\n@define-color error_color #cc0000;\n";

#[test]
fn a_light_gtk_theme_keeps_the_catalog_palette() {
    let light = ThemeColors {
        base: "#ffffff".to_owned(),
        background: "#f2f1f0".to_owned(),
        foreground: "#3c3c3c".to_owned(),
        accent: "#f07746".to_owned(),
        borders: "#c7c2be".to_owned(),
        error: "#cc0000".to_owned(),
    };
    assert_eq!(dark_tokens(&querencia_classic(), &light), None);
}

#[test]
fn a_dark_gtk_theme_supplies_the_palette_with_readable_text() {
    let tokens =
        dark_tokens(&querencia_classic(), &querencia_klassisch_dark()).expect("dark palette");

    assert_eq!(tokens.name, "Querencia Klassisch");
    assert_eq!(tokens.background, "#2b2a27");
    assert_eq!(tokens.surface, "#3c3b37");
    assert_eq!(tokens.text, "#dfdbd2");
    assert_eq!(tokens.accent, "#f07746");
    assert_eq!(tokens.border, "#23221f");
    for background in [&tokens.background, &tokens.surface] {
        for (role, foreground) in [
            ("text", &tokens.text),
            ("dim text", &tokens.dim_text),
            ("danger", &tokens.danger),
        ] {
            assert!(
                contrast(foreground, background) >= 4.5,
                "{role} {foreground} on {background}"
            );
        }
    }
    assert!(contrast(&tokens.dim_text, &tokens.muted) >= 4.5);
    for syntax in [
        &tokens.syntax_keyword,
        &tokens.syntax_string,
        &tokens.syntax_constant,
        &tokens.syntax_type,
        &tokens.syntax_preprocessor,
    ] {
        let syntax = syntax.as_deref().expect("syntax colour");
        assert!(contrast(syntax, &tokens.background) >= 4.5, "{syntax}");
    }
}

#[test]
fn querencia_klassisch_follows_a_dark_gtk_theme_while_other_themes_keep_their_colors() {
    gtk_test(
        "ui::theme::system_colors::tests::querencia_klassisch_follows_a_dark_gtk_theme_while_other_themes_keep_their_colors",
        || {
            let display = gtk::gdk::Display::default().expect("display");
            let manager = ThemeManager::shared();
            manager.select_theme(SYSTEM_THEME_ID);
            let light_surface = 0xf2_f1_f0;
            assert_eq!(manager.active_model_palette().surface, light_surface);

            let dark_theme = gtk::CssProvider::new();
            dark_theme.load_from_string(DARK_THEME_CSS);
            gtk::style_context_add_provider_for_display(
                &display,
                &dark_theme,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
            let settings = gtk::Settings::default().expect("settings");
            settings.notify("gtk-theme-name");
            wait_for(|| manager.active_model_palette().surface == 0x3c_3b_37);
            assert_eq!(
                manager.current_tokens().expect("selected theme").background,
                "#2b2a27"
            );

            manager.select_theme("0x96f");
            assert_eq!(
                manager.current_tokens().expect("selected theme").surface,
                "#262427"
            );

            manager.select_theme(SYSTEM_THEME_ID);
            gtk::style_context_remove_provider_for_display(&display, &dark_theme);
            settings.notify("gtk-theme-name");
            wait_for(|| manager.active_model_palette().surface == light_surface);
            assert_eq!(
                manager.current_tokens().expect("selected theme"),
                querencia_classic()
            );
        },
    );
}

fn wait_for(mut ready: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !ready() {
        assert!(
            std::time::Instant::now() < deadline,
            "theme colours were not applied"
        );
        gtk::glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
