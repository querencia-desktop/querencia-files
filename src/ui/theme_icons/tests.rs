// SPDX-License-Identifier: MIT

use std::path::Path;

use super::{
    CLASSIC_ICON_THEME, classic_icon_theme, file_name_icon_names, folder_icon_names,
    icon_theme_installed, path_icon_names, place_icon_names, place_path_icon_names,
};
use crate::assets::icons;

#[test]
fn files_ask_for_their_mime_icon_and_a_generic_fallback() {
    for (name, specific, generic) in [
        ("photo.jpg", "image-jpeg", "image-x-generic"),
        ("photo.JPG", "image-jpeg", "image-x-generic"),
        ("shot.png", "image-png", "image-x-generic"),
        ("notes.txt", "text-plain", "text-x-generic"),
        (
            "archive.tar.gz",
            "application-x-compressed-tar",
            "package-x-generic",
        ),
    ] {
        let names = file_name_icon_names(name);
        assert!(names.contains(&specific.to_owned()), "{name}: {names:?}");
        assert!(names.contains(&generic.to_owned()), "{name}: {names:?}");
        assert!(
            names.iter().position(|n| n == specific) < names.iter().position(|n| n == generic),
            "{name}: specific icon must come first in {names:?}"
        );
    }
}

#[test]
fn unknown_files_still_get_a_generic_icon_name() {
    let names = file_name_icon_names("blob.no-such-extension");
    assert!(!names.is_empty());
    assert!(
        names.iter().any(|name| name.ends_with("-x-generic")),
        "{names:?}"
    );
}

#[test]
fn folders_use_the_classic_folder_icon_regardless_of_name() {
    assert_eq!(folder_icon_names(), ["folder", "inode-directory"]);
    assert_eq!(
        path_icon_names(Path::new("/tmp/report.pdf"), true),
        folder_icon_names()
    );
    assert!(
        path_icon_names(Path::new("/tmp/report.pdf"), false)
            .contains(&"application-pdf".to_owned())
    );
    assert!(path_icon_names(Path::new("/"), false).is_empty());
}

#[test]
fn places_map_to_freedesktop_names_and_unknown_icons_stay_bundled() {
    assert_eq!(place_icon_names(icons::HOME), ["user-home"]);
    assert_eq!(place_icon_names(icons::TRASH), ["user-trash"]);
    assert_eq!(place_icon_names(icons::FOLDER), ["folder"]);
    assert_eq!(place_icon_names(icons::CLOCK), ["document-open-recent"]);
    assert_eq!(place_icon_names(icons::HARD_DRIVE), ["drive-harddisk"]);
    assert!(place_icon_names(icons::SEARCH).is_empty());
}

#[test]
fn the_classic_icon_theme_replaces_only_gtks_built_in_default() {
    let installed = |name: &str| name == CLASSIC_ICON_THEME;
    for default in ["", "Adwaita", "hicolor"] {
        assert_eq!(
            classic_icon_theme(default, installed),
            Some("gnome-human"),
            "{default}"
        );
    }
    // the user's own choice stays
    assert_eq!(classic_icon_theme("Papirus", installed), None);
    assert_eq!(classic_icon_theme("Humanity", installed), None);
    // not installed: nothing changes
    assert_eq!(classic_icon_theme("Adwaita", |_| false), None);
}

#[test]
fn an_icon_theme_counts_as_installed_only_with_its_index() {
    let base = std::env::temp_dir().join(format!("qf0f-icons-{}", std::process::id()));
    let dir = base.join("icons").join("gnome-human");
    std::fs::create_dir_all(&dir).expect("icon theme fixture");
    assert!(!icon_theme_installed(
        "gnome-human",
        std::slice::from_ref(&base)
    ));
    std::fs::write(dir.join("index.theme"), "[Icon Theme]\n").expect("index fixture");
    assert!(icon_theme_installed(
        "gnome-human",
        &[base.join("leer"), base.clone()]
    ));
    assert!(!icon_theme_installed(
        "Papirus",
        std::slice::from_ref(&base)
    ));
    std::fs::remove_dir_all(&base).expect("cleanup");
}

#[test]
fn sidebar_places_ask_for_their_classic_gnome_icon_first() {
    let home = gtk::glib::home_dir();
    let names = place_path_icon_names(&home, icons::HOME);
    assert_eq!(names.first().map(String::as_str), Some("user-home"));
    assert!(names.ends_with(&["folder".to_owned(), "inode-directory".to_owned()]));
    // an ordinary pinned folder: only the generic folder names
    assert_eq!(
        place_path_icon_names(Path::new("/srv/projekt"), icons::FOLDER),
        ["folder", "inode-directory"]
    );
    // every name once
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "{names:?}");
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
