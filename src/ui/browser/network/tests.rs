// SPDX-License-Identifier: MIT

use super::*;

fn schemes(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn server_addresses_are_normalized_or_rejected_with_the_cause() {
    let all = schemes(&["file", "smb", "sftp", "ftp", "dav", "network"]);
    for (input, expected) in [
        (
            "smb://192.0.2.10/freigabe",
            Ok("smb://192.0.2.10/freigabe".to_owned()),
        ),
        ("  smb://nas/freigabe ", Ok("smb://nas/freigabe".to_owned())),
        (
            "sftp://benutzer@server",
            Ok("sftp://benutzer@server".to_owned()),
        ),
        (
            "SSH://benutzer@server/home",
            Ok("sftp://benutzer@server/home".to_owned()),
        ),
        (
            r"\\nas\freigabe\ordner",
            Ok("smb://nas/freigabe/ordner".to_owned()),
        ),
        ("", Err(AddressError::Empty)),
        (
            "nas/freigabe",
            Err(AddressError::MissingScheme("smb://nas/freigabe".to_owned())),
        ),
        ("smb:///freigabe", Err(AddressError::MissingHost)),
        (
            "file:///etc",
            Err(AddressError::UnsupportedScheme("file".to_owned())),
        ),
        (
            "http://server/",
            Err(AddressError::UnsupportedScheme("http".to_owned())),
        ),
        ("smb://[nas/x", Err(AddressError::Invalid)),
    ] {
        assert_eq!(server_address(input, &all), expected, "{input}");
    }
}

#[test]
fn server_address_reports_a_missing_gvfs_backend() {
    let without_smb = schemes(&["file", "sftp"]);
    assert_eq!(
        server_address("smb://nas/freigabe", &without_smb),
        Err(AddressError::BackendMissing(
            "smb://nas/freigabe".to_owned()
        ))
    );
    assert!(
        AddressError::BackendMissing("smb://nas/freigabe".to_owned())
            .message()
            .contains("smb://")
    );
}

#[test]
fn network_root_is_recognized_in_all_gio_spellings() {
    for uri in ["network:///", "network://", "network:", "smb:///"] {
        assert!(is_network_root(&Location::uri(uri)), "{uri}");
    }
    assert!(!is_network_root(&Location::uri("smb://nas/")));
    assert!(!is_network_root(&Location::local("/network")));
}

#[test]
fn network_entries_get_workgroup_server_and_share_icons() {
    let icon = |uri: &str| network_icon_names(&Location::uri(uri)).map(|names| names[0]);
    assert_eq!(icon("smb:///"), Some("network-workgroup"));
    assert_eq!(icon("smb://nas/"), Some("network-server"));
    assert_eq!(icon("sftp://benutzer@server"), Some("network-server"));
    assert_eq!(
        icon("dns-sd://local/drucker._ipp._tcp"),
        Some("network-server")
    );
    assert_eq!(icon("smb://nas/freigabe"), Some("folder-remote"));
    assert_eq!(icon("smb://nas/freigabe/ordner"), None);
    assert_eq!(icon("sftp://server/home"), None);
    assert_eq!(icon("trash:///"), None);
    assert_eq!(network_icon_names(&Location::local("/srv")), None);
}

#[test]
fn discovery_reasons_name_each_missing_service() {
    let complete = Discovery {
        network_backend: true,
        avahi: true,
        wsdd: true,
        smb_browsing: true,
    };
    assert_eq!(complete.reasons().len(), 1);
    let bare = Discovery {
        avahi: false,
        wsdd: false,
        smb_browsing: false,
        ..complete.clone()
    };
    assert_eq!(bare.reasons().len(), 3);
    let no_backend = Discovery {
        network_backend: false,
        ..bare
    };
    assert_eq!(no_backend.reasons().len(), 1);
    assert!(no_backend.reasons()[0].contains("gvfs-backends"));
}

#[test]
fn english_gvfs_failures_get_an_explanation_and_keep_the_original() {
    let failed = |message: &str| glib::Error::new(gio::IOErrorEnum::Failed, message);
    for message in [
        "Failed to mount Windows share: Connection refused",
        "Failed to mount Windows share: Connection timed out",
        "Connection failed",
        "Failed to retrieve share list from server: No such file or directory",
        "Failed to mount Windows share: No such file or directory",
    ] {
        let explained = gvfs_failure_message(&failed(message)).expect(message);
        assert!(explained.contains(message), "{explained}");
        assert!(explained.len() > message.len() + 20, "{explained}");
    }
    assert_eq!(gvfs_failure_message(&failed("Something unusual")), None);
}

#[test]
fn gvfs_host_key_question_is_rebuilt_only_when_recognized() {
    let question = "Identity Verification Failed\nVerifying the identity of “127.0.0.1” failed, this happens when you log in to a computer the first time.\n\nThe identity sent by the remote computer is “%s”. If you want to be absolutely sure it is safe to continue, contact the system administrator.";
    let known = gvfs_question_text(&question.replace("%s", "SHA256:abc")).expect("parsed");
    assert!(
        known.contains("127.0.0.1") && known.contains("SHA256:abc"),
        "{known}"
    );
    let unknown = gvfs_question_text(&question.replace("%s", "???")).expect("parsed");
    assert!(
        unknown.contains("127.0.0.1") && !unknown.contains("???"),
        "{unknown}"
    );
    assert_eq!(gvfs_question_text("Continue?"), None);
    assert_eq!(gvfs_choice_label("Something else"), "Something else");
}

#[test]
fn gvfs_password_prompts_are_recognized() {
    let sftp =
        gvfs_prompt_text("Authentication Required\nEnter password for “benutzer” on “server”:")
            .expect("sftp");
    assert!(
        sftp.contains("benutzer") && sftp.contains("server") && !sftp.contains("Required"),
        "{sftp}"
    );
    let share = gvfs_prompt_text("Password required for share freigabe on nas").expect("share");
    assert!(
        share.contains("freigabe") && share.contains("nas"),
        "{share}"
    );
    let server = gvfs_prompt_text("Password required for nas").expect("server");
    assert!(
        server.contains("nas") && !server.contains("share"),
        "{server}"
    );
    assert_eq!(gvfs_prompt_text("Enter the passphrase for drive"), None);
}

#[test]
fn equal_gio_files_are_the_same_location() {
    assert!(same_location(
        &Location::uri("sftp://server/home"),
        &Location::uri("sftp://server/home")
    ));
    assert!(!same_location(
        &Location::uri("sftp://server/home"),
        &Location::uri("sftp://server/srv")
    ));
}
