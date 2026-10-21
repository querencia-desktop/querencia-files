// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors

//! Querencia: the Updates page while Strata's own update check is disabled
//! ([`crate::services::SELF_UPDATE`]). It only says where updates come from;
//! it has no switch, no release channel, no release notes and no network access.

use super::{control_row, page_content, scrollable_page, settings_group};
use crate::assets::icons;
use gtk::prelude::*;

pub(super) const TITLE: &str = "Updates come from the package source";
const DESCRIPTION: &str = "Querencia Files is updated through the system's package source, together with the rest of the system. It does not check for updates itself and makes no network connection for this.";

pub(super) fn package_updates_page() -> gtk::Widget {
    let content = page_content();
    content.add_css_class("settings-updates-page");
    let group = settings_group(&content, "");
    let icon = crate::assets::primary_icon(icons::INFO, 18);
    icon.set_valign(gtk::Align::Center);
    group.append(&control_row(TITLE, DESCRIPTION, &icon));
    scrollable_page(&content, None)
}
