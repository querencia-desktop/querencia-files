// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors

use std::time::{Duration, Instant};

use super::*;

fn plugged_in() -> AutomountFacts {
    AutomountFacts {
        enabled: true,
        can_mount: true,
        should_automount: true,
        mounted: false,
        network: false,
    }
}

#[test]
fn a_newly_connected_medium_is_mounted_only_when_every_condition_holds() {
    assert!(should_automount(plugged_in()));
    type Change = fn(&mut AutomountFacts);
    let blocked: [(&str, Change); 5] = [
        ("preference off", |facts| facts.enabled = false),
        ("cannot mount", |facts| facts.can_mount = false),
        ("internal volume", |facts| facts.should_automount = false),
        ("already mounted", |facts| facts.mounted = true),
        ("network volume", |facts| facts.network = true),
    ];
    for (reason, change) in blocked {
        let mut facts = plugged_in();
        change(&mut facts);
        assert!(!should_automount(facts), "{reason}");
    }
}

#[test]
fn only_device_mounts_that_the_user_did_not_just_open_are_announced() {
    let device = AnnounceFacts {
        shadowed: false,
        has_volume: true,
        network: false,
        requested_in_files: false,
    };
    assert!(should_announce(device));
    for facts in [
        AnnounceFacts {
            shadowed: true,
            ..device
        },
        AnnounceFacts {
            has_volume: false,
            ..device
        },
        AnnounceFacts {
            network: true,
            ..device
        },
        AnnounceFacts {
            requested_in_files: true,
            ..device
        },
    ] {
        assert!(!should_announce(facts), "{facts:?}");
    }
}

#[test]
fn a_mount_requested_in_files_suppresses_one_announcement_for_a_while() {
    let start = Instant::now();
    let mut mounts = UserMounts::default();
    mounts.remember("/dev/loop8".into(), start);
    assert!(!mounts.take(&["/dev/sdb1".into()], start));
    assert!(mounts.take(&["2117-D1DC".into(), "/dev/loop8".into()], start));
    assert!(
        !mounts.take(&["/dev/loop8".into()], start),
        "taken only once"
    );

    mounts.remember("/dev/loop9".into(), start);
    let later = start + USER_MOUNT_WINDOW + Duration::from_secs(1);
    assert!(
        !mounts.take(&["/dev/loop9".into()], later),
        "a failed mount must not silence the device forever"
    );
}
