// SPDX-License-Identifier: MIT

use gtk::gio;

use super::*;

fn network_entry(name: &str, display_name: &str, target: &str) -> FileEntry {
    let info = gio::FileInfo::new();
    info.set_name(name);
    info.set_display_name(display_name);
    info.set_file_type(gio::FileType::Shortcut);
    info.set_attribute_string(gio::FILE_ATTRIBUTE_STANDARD_TARGET_URI, target);
    super::super::entry_from_info(Location::uri(format!("network:///{name}")), info)
}

fn names(entries: &[FileEntry]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry.display_name.as_str())
        .collect()
}

fn target_of(entry: &FileEntry) -> &str {
    entry.location.uri_value().unwrap_or_default()
}

#[test]
fn a_device_found_by_avahi_and_wsdd_is_listed_once_with_the_dnssd_target() {
    let listing = vec![
        network_entry(
            "wsdd-server-9ac3718c-68f5-5157-a2d5-0d1f35969611",
            "NAS-TEST",
            "smb://NAS-TEST.local/",
        ),
        network_entry(
            "dnssd-server-nas-test._smb._tcp",
            "nas-test",
            "smb://192.0.2.12:445/",
        ),
        network_entry(
            "wsdd-server-544b01b0-4eb5-4855-8973-1e9f6caf922e",
            "PC-TEST",
            "smb://PC-TEST.local/",
        ),
    ];
    let dnssd_target = target_of(&listing[1]).to_owned();
    let wsdd_target = target_of(&listing[0]).to_owned();
    let (entries, merged) = merge_network_entries(listing);
    assert_eq!(names(&entries), ["nas-test", "PC-TEST"]);
    assert_eq!(target_of(&entries[0]), dnssd_target);
    let sources = &merged[target_of(&entries[0])];
    assert_eq!(
        sources
            .iter()
            .map(|source| (
                source.service,
                source.display_name.as_str(),
                source.target.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (DiscoveryService::DnsSd, "nas-test", dnssd_target.as_str()),
            (
                DiscoveryService::WsDiscovery,
                "NAS-TEST",
                wsdd_target.as_str()
            ),
        ]
    );
    assert!(!merged.contains_key(target_of(&entries[1])));
}

#[test]
fn names_match_without_case_and_local_suffix() {
    assert_eq!(device_key("NAS-TEST"), "nas-test");
    assert_eq!(device_key("nas-test.local"), "nas-test");
    assert_eq!(device_key(" NAS.Local. "), "nas");
    assert_ne!(device_key("nas2"), device_key("nas"));
}

#[test]
fn wsdd_alone_stays_and_other_protocols_or_entries_are_not_folded() {
    let listing = vec![
        network_entry("wsdd-server-1", "PRINTSRV", "smb://PRINTSRV.local/"),
        network_entry("dnssd-server-nas._smb._tcp", "nas", "smb://192.0.2.20:445/"),
        network_entry(
            "dnssd-server-nas._sftp-ssh._tcp",
            "nas",
            "sftp://192.0.2.20:22/",
        ),
        network_entry("smblink-root", "Windows Network", "smb:///"),
    ];
    let (entries, merged) = merge_network_entries(listing);
    assert_eq!(entries.len(), 4);
    assert!(merged.is_empty());
}

#[test]
fn the_listing_remembers_merged_sources_for_the_tooltip() {
    let entries = merge_network_listing(vec![
        network_entry("dnssd-server-nas._smb._tcp", "nas", "smb://192.0.2.20:445/"),
        network_entry("wsdd-server-2", "NAS", "smb://NAS.local/"),
    ]);
    let kept = entries[0].location.clone();
    assert_eq!(
        merged_network_sources(&kept).map(|sources| sources.len()),
        Some(2)
    );
    merge_network_listing(Vec::new());
    assert!(merged_network_sources(&kept).is_none());
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
