// SPDX-License-Identifier: MIT

use std::path::Path;

use super::{file_name_icon_names, folder_icon_names, path_icon_names, place_icon_names};
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
    assert!(place_icon_names(icons::SEARCH).is_empty());
}
