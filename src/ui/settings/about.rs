// SPDX-License-Identifier: MIT

use super::{page_content, scrollable_page, settings_group};
use crate::{assets::icons, services};
use gtk::prelude::*;

#[cfg(test)]
mod tests;

/// Querencia: the project website. The source code has no public home yet.
const WEBSITE: &str = "https://querencialinux.org";
/// Querencia: Strata's MIT license, embedded so that the About page shows the
/// copyright notice and permission text of the program it is based on.
const LICENSE_TEXT: &str = include_str!("../../../LICENSE");

pub(super) fn about_page() -> gtk::Widget {
    let content = page_content();
    content.add_css_class("about-page");
    let identity = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    identity.add_css_class("about-identity");
    super::search::tag(&identity, "Version information");
    let icon = app_icon();
    icon.set_pixel_size(36);
    let logo = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    logo.add_css_class("about-logo");
    icon.set_halign(gtk::Align::Center);
    icon.set_valign(gtk::Align::Center);
    icon.set_hexpand(true);
    logo.append(&icon);
    logo.set_valign(gtk::Align::Center);
    logo.set_halign(gtk::Align::Start);
    logo.set_hexpand(false);
    identity.append(&logo);
    let copy = gtk::Box::new(gtk::Orientation::Vertical, 6);
    copy.set_hexpand(true);
    copy.set_valign(gtk::Align::Center);
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let name = gtk::Label::new(Some("Querencia Files"));
    name.add_css_class("about-name");
    heading.append(&name);
    let kind = crate::build_info::build_kind();
    if kind != services::BuildKind::Stable {
        let badge = gtk::Label::new(Some(&kind.localized_label()));
        badge.add_css_class("prerelease-badge");
        badge.set_valign(gtk::Align::Center);
        heading.append(&badge);
    }
    copy.append(&heading);
    let description = gtk::Label::new(Some(&crate::i18n::tr(
        "The file manager of Querencia Desktop.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("about-description");
    copy.append(&description);
    identity.append(&copy);
    let button = gtk::Button::with_label(&crate::i18n::tr("Copy version info"));
    button.add_css_class("settings-update-check");
    button.set_valign(gtk::Align::Center);
    button.connect_clicked(|button| button.clipboard().set_text(&version_info()));
    identity.append(&button);
    content.append(&identity);

    let build = settings_group(&content, "BUILD");
    super::search::tag(&build, "Version information");
    append_about_detail(&build, "Version", crate::build_info::QUERENCIA_VERSION);
    append_about_detail(&build, "Based on", &strata_version());
    append_about_detail(&build, "Commit", crate::build_info::COMMIT);
    append_about_detail(&build, "Toolkit", &toolkit_version());
    let links = settings_group(&content, "LINKS");
    let website = gtk::LinkButton::builder().uri(WEBSITE).build();
    website.add_css_class("about-repository");
    super::search::tag(&website, "Website");
    let label = crate::i18n::tr("Website");
    crate::ui::accessibility::set_label(&website, &label);
    website.set_child(Some(&about_row(
        icons::GLOBE,
        &label,
        None,
        "",
        icons::EXTERNAL_LINK,
    )));
    links.append(&website);

    let source = gtk::Box::new(gtk::Orientation::Vertical, 0);
    source.add_css_class("about-static-row");
    super::search::tag(&source, "Source code");
    let label = crate::i18n::tr("Source code");
    crate::ui::accessibility::set_label(&source, &label);
    let pending = crate::i18n::tr("Coming with the public release");
    source.append(&about_row(icons::CODE_XML, &label, Some(&pending), "", ""));
    links.append(&source);

    links.append(&license_row());
    scrollable_page(&content, None)
}

/// Querencia: the license row names Strata as the origin and expands to the
/// embedded license text instead of linking to the upstream repository.
fn license_row() -> gtk::Box {
    let label = crate::i18n::tr("License");
    let origin = crate::i18n::tr("Based on Strata by LGSE Ltd. (MIT)");
    let toggle = gtk::Button::builder()
        .child(&about_row(
            icons::SCALE,
            &label,
            Some(&origin),
            "MIT",
            icons::CHEVRON_RIGHT,
        ))
        .build();
    toggle.add_css_class("about-repository");
    toggle.add_css_class("about-license-toggle");
    crate::ui::accessibility::set_label(&toggle, &label);
    let text = gtk::Label::new(Some(LICENSE_TEXT.trim_end()));
    text.add_css_class("about-license-text");
    text.set_xalign(0.0);
    text.set_wrap(true);
    text.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    text.set_selectable(true);
    let details = gtk::Revealer::builder()
        .child(&text)
        .transition_duration(0)
        .reveal_child(false)
        .build();
    let expander = gtk::Box::new(gtk::Orientation::Vertical, 0);
    super::search::tag(&expander, "License");
    expander.append(&toggle);
    expander.append(&details);
    toggle.update_state(&[gtk::accessible::State::Expanded(Some(false))]);
    toggle.connect_clicked(move |button| {
        let expanded = !details.reveals_child();
        details.set_reveal_child(expanded);
        button.update_state(&[gtk::accessible::State::Expanded(Some(expanded))]);
        if expanded {
            button.add_css_class("expanded");
        } else {
            button.remove_css_class("expanded");
        }
    });
    expander
}

/// One row of the LINKS group: icon, title (with an optional second line),
/// detail text and an end icon; empty `detail`/`end_icon` are left out.
fn about_row(
    icon: &str,
    title: &str,
    subtitle: Option<&str>,
    detail: &str,
    end_icon: &str,
) -> super::wrap::WrapRow {
    let row = super::wrap::WrapRow::new(16);
    row.append(&crate::assets::primary_icon(icon, 18));
    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.set_wrap(true);
    title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    text.append(&title);
    if let Some(subtitle) = subtitle {
        let subtitle = gtk::Label::new(Some(subtitle));
        subtitle.add_css_class("settings-option-description");
        subtitle.add_css_class("about-origin");
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        text.append(&subtitle);
    }
    row.append(&text);
    let metadata = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    let detail = gtk::Label::new(Some(detail));
    detail.add_css_class("settings-option-description");
    detail.add_css_class("settings-control-label");
    detail.set_visible(!detail.text().is_empty());
    metadata.append(&detail);
    if !end_icon.is_empty() {
        let end_icon = crate::assets::primary_icon(end_icon, 16);
        end_icon.add_css_class("about-end-icon");
        metadata.append(&end_icon);
    }
    row.append(&metadata);
    row
}

fn strata_version() -> String {
    format!("Strata {}", crate::build_info::installed_version())
}

/// Querencia: the application icon from the icon theme instead of the Strata
/// logo, or the theme's generic file manager icon when the theme lacks it.
fn app_icon() -> gtk::Image {
    const APP_ICON: &str = "org.querencia.Files";
    let themed = gtk::gdk::Display::default()
        .is_some_and(|display| gtk::IconTheme::for_display(&display).has_icon(APP_ICON));
    gtk::Image::from_icon_name(if themed {
        APP_ICON
    } else {
        "system-file-manager"
    })
}

fn toolkit_version() -> String {
    format!(
        "GTK {}.{}.{}",
        gtk::major_version(),
        gtk::minor_version(),
        gtk::micro_version()
    )
}

fn version_info() -> String {
    rust_i18n::t!(
        "Querencia Files %{value1}\n%{value2}\nCommit: %{value3}\nToolkit: %{value4}\nAuthor: %{value5}\nLicense: MIT\nBased on %{value6} (%{value7}, MIT)",
        value1 = crate::build_info::QUERENCIA_VERSION,
        value2 = crate::i18n::tr("The file manager of Querencia Desktop."),
        value3 = crate::build_info::COMMIT,
        value4 = toolkit_version(),
        value5 = crate::build_info::QUERENCIA_AUTHOR,
        value6 = strata_version(),
        value7 = crate::build_info::AUTHOR
    )
    .into_owned()
}

fn append_about_detail(container: &gtk::Box, label: &str, value: &str) {
    let row = super::wrap::WrapRow::new(16);
    row.set_end_align(true);
    row.add_css_class("about-detail-row");
    let label = gtk::Label::new(Some(&crate::i18n::tr(label)));
    label.add_css_class("about-detail-label");
    label.add_css_class("settings-control-label");
    label.set_xalign(0.0);
    label.set_hexpand(true);
    let value = gtk::Label::new(Some(value));
    value.add_css_class("about-detail-value");
    value.add_css_class("settings-nowrap");
    value.set_selectable(true);
    value.set_xalign(1.0);
    row.append(&label);
    row.append(&value);
    container.append(&row);
}
