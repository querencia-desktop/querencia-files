// SPDX-License-Identifier: MIT

use std::ffi::OsString;
use std::path::PathBuf;

use super::{ARCHIVE_TYPES, ArchiveTool};
use crate::model::{EntryKind, FileEntry, Location, MetadataValue};

fn entry(location: Location, name: &str, kind: EntryKind) -> FileEntry {
    FileEntry {
        thumbnail_path: None,
        location,
        native_name: OsString::from(name),
        display_name: name.into(),
        kind,
        size: MetadataValue::Unknown,
        modified_unix_seconds: MetadataValue::Unknown,
        recent_unix_seconds: MetadataValue::Unknown,
        recent_uri: None,
        is_hidden: false,
        mode: MetadataValue::Unknown,
        image_dimensions: MetadataValue::Unknown,
        child_count: MetadataValue::Unknown,
        duration_seconds: MetadataValue::Unknown,
    }
}

fn local(name: &str, kind: EntryKind) -> FileEntry {
    entry(Location::local(format!("/fixture/{name}")), name, kind)
}

fn tool(types: &[&str]) -> ArchiveTool {
    ArchiveTool::from_parts(
        None,
        Some((
            PathBuf::from("/opt/archiv/querencia-archiv"),
            types.iter().map(|value| (*value).to_owned()).collect(),
        )),
    )
    .expect("tool")
}

#[test]
fn archiv_is_used_from_path_or_its_launcher_with_the_launcher_types() {
    let in_path = PathBuf::from("/usr/bin/querencia-archiv");
    let launched = PathBuf::from("/opt/archiv/querencia-archiv");
    assert_eq!(ArchiveTool::from_parts(None, None), None);
    let only_path = ArchiveTool::from_parts(Some(in_path.clone()), None).expect("in PATH");
    assert_eq!(only_path.program, in_path);
    assert_eq!(only_path.types, ARCHIVE_TYPES);
    let both = ArchiveTool::from_parts(
        Some(in_path.clone()),
        Some((launched.clone(), vec!["application/zip".to_owned()])),
    )
    .expect("both");
    assert_eq!(
        (both.program, both.types),
        (in_path, vec!["application/zip".to_owned()])
    );
    let launcher =
        ArchiveTool::from_parts(None, Some((launched.clone(), Vec::new()))).expect("launcher");
    assert_eq!(
        (launcher.program, launcher.types.len()),
        (launched, ARCHIVE_TYPES.len())
    );
}

#[test]
fn only_local_archive_files_of_a_listed_type_are_extracted_by_archiv() {
    let archiv = tool(&ARCHIVE_TYPES);
    for name in [
        "bericht.zip",
        "bericht.tar.gz",
        "bericht.tar.xz",
        "bericht.7z",
        "notiz.txt.gz",
    ] {
        assert!(archiv.extracts(&local(name, EntryKind::File)), "{name}");
    }
    for (candidate, why) in [
        (local("brief.odt", EntryKind::File), "zip subtype"),
        (local("notiz.txt", EntryKind::File), "no archive"),
        (local("ordner.zip", EntryKind::Directory), "folder"),
        (
            entry(
                Location::uri("sftp://server-test/bericht.zip"),
                "bericht.zip",
                EntryKind::File,
            ),
            "remote",
        ),
    ] {
        assert!(!archiv.extracts(&candidate), "{why}");
    }
    assert!(!tool(&["application/zip"]).extracts(&local("bericht.tar.gz", EntryKind::File)));
}

#[test]
fn extract_here_names_only_the_archive() {
    let args: Vec<String> = tool(&ARCHIVE_TYPES)
        .extract_here_command(std::path::Path::new("/fixture/bericht.zip"))
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert_eq!(args, ["--entpacken-hier", "/fixture/bericht.zip"]);
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
