// SPDX-License-Identifier: MIT

use gtk::gio;

#[test]
fn bundled_app_icon_resolves_when_the_icon_theme_lacks_it() {
    gio::resources_register_include!("strata.gresource").expect("bundled resources");
    let icon = gio::resources_lookup_data(super::BUNDLED_APP_ICON, gio::ResourceLookupFlags::NONE)
        .expect("bundled Querencia Files icon");
    assert!(icon.starts_with(b"<?xml"));
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
