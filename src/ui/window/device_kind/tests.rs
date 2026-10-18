// SPDX-License-Identifier: MIT

use super::*;

fn facts(icons: &[&str]) -> DeviceFacts {
    DeviceFacts {
        icon_names: icons.iter().map(|name| (*name).to_owned()).collect(),
        ..DeviceFacts::default()
    }
}

#[test]
fn gvfs_icon_names_map_to_the_device_kind() {
    let cases: &[(&[&str], DeviceKind)] = &[
        (
            &["drive-removable-media-usb", "drive-removable-media"],
            DeviceKind::Removable,
        ),
        (
            &["drive-removable-media", "drive-removable"],
            DeviceKind::Removable,
        ),
        (&["media-flash-sd", "media-flash"], DeviceKind::Removable),
        (
            &["drive-harddisk-usb", "drive-harddisk"],
            DeviceKind::HardDisk,
        ),
        (
            &["drive-harddisk-solidstate-symbolic", "drive-symbolic"],
            DeviceKind::HardDisk,
        ),
        (
            &["media-optical-cd-rom", "media-optical", "drive-optical"],
            DeviceKind::Optical,
        ),
        (&["folder-remote", "folder"], DeviceKind::Network),
        (&["drive"], DeviceKind::HardDisk),
    ];
    for (icons, expected) in cases {
        assert_eq!(device_kind(&facts(icons)), *expected, "{icons:?}");
    }
}

#[test]
fn network_and_encryption_outrank_the_icon() {
    let mut encrypted = facts(&["drive-removable-media-usb"]);
    encrypted.encrypted = true;
    assert_eq!(device_kind(&encrypted), DeviceKind::Encrypted);
    let mut network = facts(&["drive-harddisk"]);
    network.network = true;
    network.encrypted = true;
    assert_eq!(device_kind(&network), DeviceKind::Network);
}

#[test]
fn a_removable_drive_without_a_telling_icon_is_removable() {
    let mut plain = facts(&["drive"]);
    plain.drive_removable = true;
    assert_eq!(device_kind(&plain), DeviceKind::Removable);
}

#[test]
fn network_uris_are_recognised_by_scheme() {
    for uri in [
        "smb://nas-test.local/user/",
        "SFTP://host/",
        "davs://example.org/",
        "nfs://nas/export",
    ] {
        assert!(is_network_uri(uri), "{uri}");
    }
    for uri in [
        "file:///run/media/user/cidata",
        "trash:///",
        "network:///",
    ] {
        assert!(!is_network_uri(uri), "{uri}");
    }
}

#[test]
fn usb_sticks_get_their_own_notification_noun() {
    let stick = facts(&["drive-removable-media-usb"]);
    assert!(is_usb(&stick));
    assert_eq!(
        mounted_message_key(device_kind(&stick), is_usb(&stick)),
        "USB stick “%{name}” mounted"
    );
    let card = facts(&["media-flash-sd"]);
    assert_eq!(
        mounted_message_key(device_kind(&card), is_usb(&card)),
        "Removable medium “%{name}” mounted"
    );
}
