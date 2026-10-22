// SPDX-License-Identifier: MIT

//! Querencia: one watcher per Files process. It mounts newly connected media
//! when "Mount removable media automatically" is on and announces new device
//! mounts through `org.freedesktop.Notifications` with an "Open" action.
//! Without a running Files nothing is mounted or announced; see
//! `docs/querencia-devices.md`.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use gtk::{gdk_pixbuf, gio, glib, prelude::*};

use crate::{adapters::location_for_file, model::Location, ui::preferences::PreferenceManager};

use super::device_kind::{self, DeviceKind};

const NOTIFICATIONS_NAME: &str = "org.freedesktop.Notifications";
const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";
const OPEN_ACTION: &str = "open";
const DEFAULT_ACTION: &str = "default";
const AUTOMOUNT_SETTLE: Duration = Duration::from_millis(400);
const USER_MOUNT_WINDOW: Duration = Duration::from_secs(120);
const NOTIFICATION_ICON_SIZE: i32 = 48;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AutomountFacts {
    pub enabled: bool,
    pub can_mount: bool,
    pub should_automount: bool,
    pub mounted: bool,
    pub network: bool,
}

/// GVfs leaves `should_automount` off for system and internal volumes, so
/// plugging in a stick mounts it while a second internal disk stays alone.
pub(super) fn should_automount(facts: AutomountFacts) -> bool {
    facts.enabled && facts.can_mount && facts.should_automount && !facts.mounted && !facts.network
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct AnnounceFacts {
    pub shadowed: bool,
    pub has_volume: bool,
    pub network: bool,
    pub requested_in_files: bool,
}

/// Mounts without a volume are portal or FUSE helpers and network shares
/// were opened on purpose; a mount the user just asked for in a Files window
/// is already on screen.
pub(super) fn should_announce(facts: AnnounceFacts) -> bool {
    !facts.shadowed && facts.has_volume && !facts.network && !facts.requested_in_files
}

#[derive(Default)]
pub(super) struct UserMounts {
    requested: Vec<(String, Instant)>,
}

impl UserMounts {
    pub(super) fn remember(&mut self, identity: String, now: Instant) {
        self.forget_expired(now);
        self.requested.retain(|(known, _)| *known != identity);
        self.requested.push((identity, now));
    }

    pub(super) fn take(&mut self, identities: &[String], now: Instant) -> bool {
        self.forget_expired(now);
        let before = self.requested.len();
        self.requested
            .retain(|(known, _)| !identities.iter().any(|identity| identity == known));
        self.requested.len() != before
    }

    fn forget_expired(&mut self, now: Instant) {
        self.requested
            .retain(|(_, at)| now.saturating_duration_since(*at) < USER_MOUNT_WINDOW);
    }
}

thread_local! {
    static WATCH: RefCell<Option<DeviceWatch>> = const { RefCell::new(None) };
    static USER_MOUNTS: RefCell<UserMounts> = RefCell::new(UserMounts::default());
}

struct DeviceWatch {
    _monitor: gio::VolumeMonitor,
    _handlers: Vec<glib::SignalHandlerId>,
    _notifier: Option<Rc<Notifier>>,
}

fn volume_identities(volume: &gio::Volume) -> Vec<String> {
    let mut identities: Vec<String> = super::gio_volume_identity(volume).into_iter().collect();
    if let Some(drive) = volume.drive().as_ref().and_then(super::gio_drive_identity) {
        identities.push(drive);
    }
    identities
}

/// Called before Files mounts a device on the user's request.
pub(in crate::ui) fn note_user_mount_volume(volume: &gio::Volume) {
    let now = Instant::now();
    USER_MOUNTS.with(|mounts| {
        let mut mounts = mounts.borrow_mut();
        for identity in volume_identities(volume) {
            mounts.remember(identity, now);
        }
    });
}

pub(in crate::ui) fn note_user_mount_drive(drive: &gio::Drive) {
    if let Some(identity) = super::gio_drive_identity(drive) {
        USER_MOUNTS.with(|mounts| mounts.borrow_mut().remember(identity, Instant::now()));
    }
}

pub fn start(application: &gtk::Application) {
    if WATCH.with(|watch| watch.borrow().is_some()) {
        return;
    }
    let monitor = gio::VolumeMonitor::get();
    let notifier = application.dbus_connection().map(Notifier::new);
    let mut handlers = vec![monitor.connect_volume_added(|_, volume| {
        schedule_automount(volume.clone());
    })];
    if let Some(notifier) = notifier.clone() {
        handlers.push(monitor.connect_mount_added(move |_, mount| {
            announce_mount(&notifier, mount);
        }));
    }
    WATCH.with(|watch| {
        watch.replace(Some(DeviceWatch {
            _monitor: monitor,
            _handlers: handlers,
            _notifier: notifier,
        }));
    });
}

fn schedule_automount(volume: gio::Volume) {
    // A freshly added volume can still be settling (partition probing, a
    // competing mount); decide on its state a moment later.
    glib::timeout_add_local_once(AUTOMOUNT_SETTLE, move || {
        let facts = AutomountFacts {
            enabled: PreferenceManager::shared().automount_media(),
            can_mount: volume.can_mount(),
            should_automount: volume.should_automount(),
            mounted: volume.get_mount().is_some(),
            network: device_kind::volume_facts(&volume).network,
        };
        if !should_automount(facts) {
            tracing::debug!(volume = %volume.name(), ?facts, "volume not mounted automatically");
            return;
        }
        let Some(view) = super::active_browser_view() else {
            tracing::debug!(volume = %volume.name(), "no Files window to mount from");
            return;
        };
        tracing::info!(volume = %volume.name(), "mounting newly connected volume");
        view.automount_volume(volume);
    });
}

fn announce_mount(notifier: &Rc<Notifier>, mount: &gio::Mount) {
    let volume = mount.volume();
    let facts = device_kind::mount_facts(mount);
    let requested_in_files = volume.as_ref().is_some_and(|volume| {
        let identities = volume_identities(volume);
        USER_MOUNTS.with(|mounts| mounts.borrow_mut().take(&identities, Instant::now()))
    });
    let announce = AnnounceFacts {
        shadowed: mount.is_shadowed(),
        has_volume: volume.is_some(),
        network: facts.network,
        requested_in_files,
    };
    if !should_announce(announce) {
        return;
    }
    let Some(location) = location_for_file(&mount.root()) else {
        return;
    };
    let kind = device_kind::device_kind(&facts);
    notifier.notify(kind, device_kind::is_usb(&facts), &mount.name(), location);
}

struct Notifier {
    connection: gio::DBusConnection,
    pending: RefCell<HashMap<u32, Location>>,
    subscription: RefCell<Option<gio::SignalSubscription>>,
}

impl Notifier {
    fn new(connection: gio::DBusConnection) -> Rc<Self> {
        let notifier = Rc::new(Self {
            connection,
            pending: RefCell::new(HashMap::new()),
            subscription: RefCell::new(None),
        });
        let weak = Rc::downgrade(&notifier);
        let subscription = notifier.connection.subscribe_to_signal(
            None,
            Some(NOTIFICATIONS_NAME),
            None,
            Some(NOTIFICATIONS_PATH),
            None,
            gio::DBusSignalFlags::NONE,
            move |signal| {
                let Some(notifier) = weak.upgrade() else {
                    return;
                };
                match signal.signal_name {
                    "ActionInvoked" => {
                        if let Some((id, action)) = signal.parameters.get::<(u32, String)>() {
                            notifier.action_invoked(id, &action);
                        }
                    }
                    "NotificationClosed" => {
                        if let Some((id, _reason)) = signal.parameters.get::<(u32, u32)>() {
                            notifier.pending.borrow_mut().remove(&id);
                        }
                    }
                    _ => {}
                }
            },
        );
        notifier.subscription.replace(Some(subscription));
        notifier
    }

    fn notify(self: &Rc<Self>, kind: DeviceKind, usb: bool, name: &str, location: Location) {
        let summary =
            rust_i18n::t!(device_kind::mounted_message_key(kind, usb), name = name).into_owned();
        let body = location.display_path();
        let open = crate::i18n::tr("Open");
        let actions = vec![
            DEFAULT_ACTION.to_owned(),
            open.clone(),
            OPEN_ACTION.to_owned(),
            open,
        ];
        let mut hints: HashMap<String, glib::Variant> = HashMap::from([
            (
                "desktop-entry".to_owned(),
                crate::APPLICATION_ID.to_variant(),
            ),
            ("category".to_owned(), "device.added".to_variant()),
        ]);
        if let Some(image) = notification_image(device_kind::icon_name(kind)) {
            hints.insert("image-data".to_owned(), image);
        }
        let parameters = (
            "Querencia Files".to_owned(),
            0u32,
            String::new(),
            summary,
            body,
            actions,
            hints,
            -1i32,
        )
            .to_variant();
        let weak = Rc::downgrade(self);
        self.connection.call(
            Some(NOTIFICATIONS_NAME),
            NOTIFICATIONS_PATH,
            NOTIFICATIONS_NAME,
            "Notify",
            Some(&parameters),
            Some(glib::VariantTy::new("(u)").expect("valid reply type")),
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
            move |reply| match reply {
                Ok(reply) => {
                    if let (Some(notifier), Some((id,))) = (weak.upgrade(), reply.get::<(u32,)>()) {
                        notifier.pending.borrow_mut().insert(id, location);
                    }
                }
                Err(error) => {
                    tracing::info!(%error, "mount notification not shown");
                }
            },
        );
    }

    fn action_invoked(&self, id: u32, action: &str) {
        if action != OPEN_ACTION && action != DEFAULT_ACTION {
            return;
        }
        let Some(location) = self.pending.borrow_mut().remove(&id) else {
            return;
        };
        super::open_location_in_new_window(location);
    }
}

fn notification_image(icon: &str) -> Option<glib::Variant> {
    let path = format!("/org/querencia/Files/icons/scalable/devices/{icon}.svg");
    let pixbuf = gdk_pixbuf::Pixbuf::from_resource_at_scale(
        &path,
        NOTIFICATION_ICON_SIZE,
        NOTIFICATION_ICON_SIZE,
        true,
    )
    .ok()?;
    let pixbuf = if pixbuf.has_alpha() {
        pixbuf
    } else {
        pixbuf.add_alpha(false, 0, 0, 0).ok()?
    };
    Some(
        (
            pixbuf.width(),
            pixbuf.height(),
            pixbuf.rowstride(),
            true,
            pixbuf.bits_per_sample(),
            pixbuf.n_channels(),
            pixbuf.read_pixel_bytes().to_vec(),
        )
            .to_variant(),
    )
}

#[cfg(test)]
mod tests;
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
