// SPDX-License-Identifier: MIT

//! Querencia: which kind of device a sidebar row, notification or desktop
//! icon stands for, decided from the GIO facts that GVfs publishes.

use gtk::{gio, prelude::*};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ui) enum DeviceKind {
    Removable,
    HardDisk,
    Optical,
    Network,
    Encrypted,
}

#[derive(Clone, Debug, Default)]
pub(in crate::ui) struct DeviceFacts {
    pub icon_names: Vec<String>,
    pub encrypted: bool,
    pub network: bool,
    pub drive_removable: bool,
}

const NETWORK_SCHEMES: [&str; 9] = [
    "smb", "sftp", "ftp", "ftps", "dav", "davs", "afp", "nfs", "ssh",
];

pub(in crate::ui) fn device_kind(facts: &DeviceFacts) -> DeviceKind {
    if facts.network {
        return DeviceKind::Network;
    }
    if facts.encrypted {
        return DeviceKind::Encrypted;
    }
    let stems: Vec<String> = facts
        .icon_names
        .iter()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.strip_suffix("-symbolic").unwrap_or(&lower).to_owned()
        })
        .collect();
    let any = |prefixes: &[&str]| {
        stems
            .iter()
            .any(|stem| prefixes.iter().any(|prefix| stem.starts_with(prefix)))
    };
    if any(&["media-optical", "drive-optical"]) {
        DeviceKind::Optical
    } else if any(&["folder-remote", "network-"]) {
        DeviceKind::Network
    } else if any(&[
        "media-flash",
        "media-removable",
        "drive-removable-media",
        "media-floppy",
        "media-tape",
        "multimedia-player",
        "phone",
        "camera",
    ]) {
        DeviceKind::Removable
    } else if any(&["drive-harddisk"]) {
        DeviceKind::HardDisk
    } else if facts.drive_removable {
        DeviceKind::Removable
    } else {
        DeviceKind::HardDisk
    }
}

pub(in crate::ui) fn is_usb(facts: &DeviceFacts) -> bool {
    facts
        .icon_names
        .iter()
        .any(|name| name.to_ascii_lowercase().contains("usb"))
}

pub(in crate::ui) fn is_network_uri(uri: &str) -> bool {
    uri.split_once("://").is_some_and(|(scheme, _)| {
        NETWORK_SCHEMES
            .iter()
            .any(|known| scheme.eq_ignore_ascii_case(known))
    })
}

pub(in crate::ui) fn icon_name(kind: DeviceKind) -> &'static str {
    use crate::assets::icons;
    match kind {
        DeviceKind::Removable => icons::DEVICE_REMOVABLE,
        DeviceKind::HardDisk => icons::DEVICE_HARD_DISK,
        DeviceKind::Optical => icons::DEVICE_OPTICAL,
        DeviceKind::Network => icons::DEVICE_NETWORK,
        DeviceKind::Encrypted => icons::DEVICE_ENCRYPTED,
    }
}

/// The noun the mount notification starts with, as an i18n key.
pub(in crate::ui) fn mounted_message_key(kind: DeviceKind, usb: bool) -> &'static str {
    match kind {
        DeviceKind::Removable if usb => "USB stick “%{name}” mounted",
        DeviceKind::Removable => "Removable medium “%{name}” mounted",
        DeviceKind::HardDisk => "Hard disk “%{name}” mounted",
        DeviceKind::Optical => "Disc “%{name}” mounted",
        DeviceKind::Network => "Network drive “%{name}” connected",
        DeviceKind::Encrypted => "Encrypted volume “%{name}” mounted",
    }
}

fn icon_names(icon: &gio::Icon) -> Vec<String> {
    super::gio_icon_names(icon)
}

pub(in crate::ui) fn volume_facts(volume: &gio::Volume) -> DeviceFacts {
    let mut icon_names = icon_names(&volume.icon());
    if let Some(drive) = volume.drive() {
        icon_names.extend(icon_names_of_drive(&drive));
    }
    let network = volume
        .identifier(gio::VOLUME_IDENTIFIER_KIND_CLASS.as_str())
        .is_some_and(|class| class == "network")
        || volume
            .activation_root()
            .is_some_and(|root| is_network_uri(&root.uri()));
    DeviceFacts {
        icon_names,
        encrypted: super::gio_volume_is_encrypted(volume),
        network,
        drive_removable: volume
            .drive()
            .is_some_and(|drive| drive.is_removable() || drive.is_media_removable()),
    }
}

pub(in crate::ui) fn mount_facts(mount: &gio::Mount) -> DeviceFacts {
    if let Some(volume) = mount.volume() {
        let mut facts = volume_facts(&volume);
        facts.network |= is_network_uri(&mount.root().uri());
        return facts;
    }
    let mut icon_names = icon_names(&mount.icon());
    if let Some(drive) = mount.drive() {
        icon_names.extend(icon_names_of_drive(&drive));
    }
    DeviceFacts {
        icon_names,
        encrypted: super::gio_mount_is_encrypted(mount),
        network: is_network_uri(&mount.root().uri()),
        drive_removable: mount
            .drive()
            .is_some_and(|drive| drive.is_removable() || drive.is_media_removable()),
    }
}

pub(in crate::ui) fn drive_facts(drive: &gio::Drive) -> DeviceFacts {
    DeviceFacts {
        icon_names: icon_names_of_drive(drive),
        encrypted: true,
        network: false,
        drive_removable: drive.is_removable() || drive.is_media_removable(),
    }
}

fn icon_names_of_drive(drive: &gio::Drive) -> Vec<String> {
    icon_names(&drive.icon())
}

#[cfg(test)]
mod tests;
