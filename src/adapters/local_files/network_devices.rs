// SPDX-License-Identifier: MIT

//! Querencia: GVfs lists a device once per discovery service, for example
//! `nas-test` from DNS-SD (Avahi) and `NAS-TEST` from WS-Discovery (wsdd).
//! The network view shows one entry per device and protocol; the DNS-SD
//! target is preferred because it carries the resolved address.

use std::{cell::RefCell, collections::HashMap};

use crate::model::{FileEntry, Location};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiscoveryService {
    DnsSd,
    WsDiscovery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NetworkSource {
    pub service: DiscoveryService,
    pub display_name: String,
    pub target: String,
}

thread_local! {
    static MERGED_SOURCES: RefCell<HashMap<String, Vec<NetworkSource>>> =
        RefCell::new(HashMap::new());
}

fn discovery_service(native_name: &str) -> Option<DiscoveryService> {
    if native_name.starts_with("dnssd-") {
        Some(DiscoveryService::DnsSd)
    } else if native_name.starts_with("wsdd-") {
        Some(DiscoveryService::WsDiscovery)
    } else {
        None
    }
}

pub(crate) fn device_key(display_name: &str) -> String {
    let lower = display_name.trim().trim_end_matches('.').to_lowercase();
    lower
        .strip_suffix(".local")
        .map(str::to_owned)
        .unwrap_or(lower)
}

fn scheme(location: &Location) -> String {
    location
        .uri_value()
        .and_then(|uri| uri.split_once(':'))
        .map(|(scheme, _)| scheme.to_ascii_lowercase())
        .unwrap_or_default()
}

fn target(location: &Location) -> String {
    location
        .uri_value()
        .map(str::to_owned)
        .unwrap_or_else(|| location.display_path())
}

/// Returns the merged entries and, per kept entry's target, every source
/// that was folded into it.
pub(crate) fn merge_network_entries(
    entries: Vec<FileEntry>,
) -> (Vec<FileEntry>, HashMap<String, Vec<NetworkSource>>) {
    let mut kept: Vec<FileEntry> = Vec::with_capacity(entries.len());
    let mut groups: HashMap<(String, String), (usize, Vec<NetworkSource>)> = HashMap::new();
    for entry in entries {
        let Some(service) = discovery_service(&entry.native_name.to_string_lossy()) else {
            kept.push(entry);
            continue;
        };
        let source = NetworkSource {
            service,
            display_name: entry.display_name.clone(),
            target: target(&entry.location),
        };
        let key = (device_key(&entry.display_name), scheme(&entry.location));
        match groups.get_mut(&key) {
            Some((index, sources)) => {
                let replaces = service == DiscoveryService::DnsSd
                    && sources
                        .iter()
                        .all(|known| known.service != DiscoveryService::DnsSd);
                if replaces {
                    kept[*index] = entry;
                }
                sources.push(source);
            }
            None => {
                groups.insert(key, (kept.len(), vec![source]));
                kept.push(entry);
            }
        }
    }
    let merged = groups
        .into_values()
        .filter(|(_, sources)| sources.len() > 1)
        .map(|(index, mut sources)| {
            sources.sort_by_key(|source| source.service != DiscoveryService::DnsSd);
            (target(&kept[index].location), sources)
        })
        .collect();
    (kept, merged)
}

/// Merges a complete `network:///` listing and remembers the sources for
/// the view's tooltip.
pub(crate) fn merge_network_listing(entries: Vec<FileEntry>) -> Vec<FileEntry> {
    let (entries, merged) = merge_network_entries(entries);
    MERGED_SOURCES.with(|sources| *sources.borrow_mut() = merged);
    entries
}

pub(crate) fn merged_network_sources(location: &Location) -> Option<Vec<NetworkSource>> {
    let target = location.uri_value()?;
    MERGED_SOURCES.with(|sources| sources.borrow().get(target).cloned())
}

#[cfg(test)]
mod tests;
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
