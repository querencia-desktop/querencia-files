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
    assert_eq!(icon("smb:///"), Some(icons::NETWORK_WORKGROUP));
    assert_eq!(icon("smb://nas/"), Some(icons::NETWORK_SERVER));
    assert_eq!(icon("sftp://benutzer@server"), Some(icons::NETWORK_SERVER));
    assert_eq!(
        icon("dns-sd://local/drucker._ipp._tcp"),
        Some(icons::NETWORK_SERVER)
    );
    assert_eq!(icon("smb://nas/freigabe"), Some(icons::NETWORK_SHARE));
    assert_eq!(
        network_icon_names(&Location::uri("smb://nas/freigabe")).map(|names| names[1]),
        Some("folder-remote"),
        "the classic theme name stays as fallback"
    );
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
fn english_and_german_gvfs_failures_get_an_explanation_and_keep_the_original() {
    let failed = |message: &str| glib::Error::new(gio::IOErrorEnum::Failed, message);
    for message in [
        "Failed to mount Windows share: Connection refused",
        "Failed to mount Windows share: Connection timed out",
        "Connection failed",
        "Failed to retrieve share list from server: No such file or directory",
        "Failed to mount Windows share: No such file or directory",
        "Failed to mount Windows share: Verbindungsaufbau abgelehnt",
        "Failed to mount Windows share: Die Wartezeit für die Verbindung ist abgelaufen",
        "Verbindung fehlgeschlagen",
        "Empfangen der Freigabenliste vom Server ist gescheitert: Datei oder Verzeichnis nicht gefunden",
        "Failed to mount Windows share: Keine Route zum Zielrechner",
        "Fehler beim Auflösen von »nas.local«: Der Name oder der Dienst ist nicht bekannt",
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
fn german_gvfs_host_key_question_is_rebuilt_like_the_english_one() {
    let english = "Identity Verification Failed\nVerifying the identity of “127.0.0.1” failed, this happens when you log in to a computer the first time.\n\nThe identity sent by the remote computer is “%s”. If you want to be absolutely sure it is safe to continue, contact the system administrator.";
    let german = "Überprüfung der Identität fehlgeschlagen\nDie Identität von »127.0.0.1« konnte nicht überprüft werden. Dies ist immer der Fall, wenn Sie sich das erste Mal an einem Rechner anmelden.\n\nDie vom entfernten Rechner übermittelte Identität lautet »%s«. Wenn Sie vor dem Fortfahren absolut sicher gehen wollen, dann kontaktieren Sie Ihren Systemverwalter.";
    for fingerprint in ["SHA256:abc", "???"] {
        assert_eq!(
            gvfs_question_text(&german.replace("%s", fingerprint)),
            gvfs_question_text(&english.replace("%s", fingerprint)),
            "{fingerprint}"
        );
    }
}

#[test]
fn unrecognized_gvfs_host_key_question_keeps_its_text_and_notes_a_missing_fingerprint() {
    let french = "Échec de vérification d’identité\nL’identité envoyée par l’ordinateur distant est « ??? ».";
    let shown = gvfs_question_text(french).expect("missing fingerprint");
    assert!(shown.starts_with(french), "{shown}");
    assert!(shown.len() > french.len() + 20, "{shown}");
}

#[test]
fn gvfs_choices_put_the_safe_choice_first_and_keep_the_reply_index() {
    let choices = |labels: &[&str]| -> Vec<(usize, String, bool)> {
        gvfs_choices(&schemes(labels))
            .into_iter()
            .map(|choice| (choice.index, choice.label, choice.rejects))
            .collect()
    };
    let log_in = crate::i18n::tr("Log In Anyway");
    let cancel = crate::i18n::tr("Cancel Login");
    let host_key = vec![(1, cancel.clone(), true), (0, log_in.clone(), false)];
    assert_eq!(choices(&["Log In Anyway", "Cancel Login"]), host_key);
    assert_eq!(
        choices(&["Dennoch anmelden", "Anmelden abbrechen"]),
        host_key
    );
    assert_eq!(
        choices(&["Se connecter quand même", "Annuler la connexion"]),
        vec![
            (1, "Annuler la connexion".to_owned(), true),
            (0, "Se connecter quand même".to_owned(), false),
        ]
    );
    assert_eq!(
        choices(&["Ja", "Nein"]),
        vec![(1, "Nein".to_owned(), true), (0, "Ja".to_owned(), false)]
    );
    assert_eq!(
        choices(&["Abbrechen", "Trotzdem aushängen"]),
        vec![
            (0, "Abbrechen".to_owned(), true),
            (1, "Trotzdem aushängen".to_owned(), false),
        ]
    );
    assert_eq!(
        choices(&["Continue"]),
        vec![(0, "Continue".to_owned(), false)]
    );
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

#[test]
fn the_udisks_passphrase_prompt_is_rewritten() {
    assert_eq!(
        gvfs_prompt_text(
            "Authentication Required\nA passphrase is needed to access encrypted data on “/dev/sdb1”."
        )
        .as_deref(),
        Some("Enter the passphrase to unlock the encrypted data on “/dev/sdb1”.")
    );
}
