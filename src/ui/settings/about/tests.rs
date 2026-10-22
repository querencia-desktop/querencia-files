// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors

use super::*;

fn descendants(widget: &gtk::Widget, found: &mut Vec<gtk::Widget>) {
    let mut child = widget.first_child();
    while let Some(current) = child {
        found.push(current.clone());
        descendants(&current, found);
        child = current.next_sibling();
    }
}

fn page_widgets() -> Vec<gtk::Widget> {
    let mut found = Vec::new();
    descendants(&about_page(), &mut found);
    found
}

#[test]
fn querencia_version_info_names_querencia_files_and_the_strata_origin() {
    crate::test_support::gtk_test(
        "ui::settings::about::tests::querencia_version_info_names_querencia_files_and_the_strata_origin",
        || {
            rust_i18n::set_locale("en");
            let info = version_info();
            let lines: Vec<&str> = info.lines().collect();
            assert_eq!(
                lines[0],
                format!(
                    "Querencia Files {} ({})",
                    crate::build_info::QUERENCIA_PACKAGE_VERSION,
                    crate::build_info::QUERENCIA_RELEASE
                )
            );
            assert_eq!(lines[1], "The file manager of Querencia Desktop.");
            assert_eq!(
                lines[2],
                format!("Build: {}", crate::build_info::QUERENCIA_VERSION)
            );
            assert!(
                lines.contains(&"Author: Querencia Desktop contributors"),
                "{info}"
            );
            assert!(lines.contains(&"License: MIT"), "{info}");
            assert_eq!(
                lines.last().copied(),
                Some(
                    format!(
                        "Based on Strata {} (LGSE Ltd., MIT)",
                        crate::build_info::installed_version()
                    )
                    .as_str()
                )
            );
            rust_i18n::set_locale("de");
            let info = version_info();
            assert!(info.starts_with("Querencia Files "), "{info}");
            assert!(
                info.contains("\nDer Dateimanager von Querencia Desktop.\n"),
                "{info}"
            );
            assert!(
                info.contains("\nAutor: Querencia Desktop contributors\n"),
                "{info}"
            );
            assert!(info.ends_with("(LGSE Ltd., MIT)"), "{info}");
            assert!(info.contains("\nBasiert auf Strata "), "{info}");
            assert!(
                info.contains(&format!(
                    "\nBuild: {}\n",
                    crate::build_info::QUERENCIA_VERSION
                )),
                "{info}"
            );
        },
    );
}

#[test]
fn querencia_about_page_links_the_website_and_source_and_names_strata_as_origin() {
    crate::test_support::gtk_test(
        "ui::settings::about::tests::querencia_about_page_links_the_website_and_source_and_names_strata_as_origin",
        || {
            rust_i18n::set_locale("en");
            let widgets = page_widgets();
            let uris: Vec<String> = widgets
                .iter()
                .filter_map(|widget| widget.downcast_ref::<gtk::LinkButton>())
                .map(|button| button.uri().to_string())
                .collect();
            assert_eq!(
                uris,
                [
                    "https://querencialinux.org",
                    "https://github.com/querencia-desktop/querencia-files"
                ]
            );
            let labels: Vec<String> = widgets
                .iter()
                .filter_map(|widget| widget.downcast_ref::<gtk::Label>())
                .map(|label| label.text().to_string())
                .collect();
            assert!(labels.iter().any(|text| text == "Querencia Files"));
            assert!(
                labels
                    .iter()
                    .any(|text| text == "The file manager of Querencia Desktop.")
            );
            assert!(labels.iter().any(|text| *text
                == format!(
                    "{} ({})",
                    crate::build_info::QUERENCIA_PACKAGE_VERSION,
                    crate::build_info::QUERENCIA_RELEASE
                )));
            assert!(
                !labels
                    .iter()
                    .any(|text| text.contains(crate::build_info::QUERENCIA_VERSION)),
                "the build stage belongs only in the copied version information"
            );
            for text in labels.iter().filter(|text| text.contains("Strata")) {
                assert!(
                    text == "Based on Strata by LGSE Ltd. (MIT)"
                        || *text == format!("Strata {}", crate::build_info::installed_version())
                        || text.starts_with("MIT License\n\nCopyright (c) 2026 LGSE Ltd."),
                    "unexpected Strata text: {text}"
                );
            }
            assert!(
                labels
                    .iter()
                    .any(|text| text.contains("Copyright (c) 2026 LGSE Ltd.")
                        && text.contains("Permission is hereby granted")),
                "the embedded license text is missing"
            );
        },
    );
}
