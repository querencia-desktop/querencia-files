// SPDX-License-Identifier: MIT

//! QF-0m: classic network browsing on top of GVfs. Discovery stays GVfs'
//! `network:///`; this adds the "Connect to Server" dialog, an explanatory
//! page when nothing was discovered, server/share icons and readable GVfs
//! failures. No protocol is implemented here: every connection is a GIO
//! mount through the browser's existing mount and authentication flow.

use gtk::prelude::*;
use gtk::{gio, glib};

use crate::model::Location;
use crate::services::{backend_unavailable_message, sanitize_failure_message};
use crate::ui::browser::BrowserView;
use crate::ui::browser::location::TypedLocation;
use crate::ui::controls::{dialog_text, form_entry, form_error_label, form_label, modal_layout};
use crate::ui::modal::{
    ModalHost, close_on_escape, dismiss_modal_layer, modal_layer, show_error_dialog,
    submit_on_enter,
};

#[cfg(test)]
mod tests;

pub(in crate::ui) const NETWORK_URI: &str = "network:///";
/// The classic chrome's action; the empty network page's button activates it.
pub(in crate::ui) const CONNECT_ACTION: &str = "classic.connect-to-server";
const NETWORK_PAGE: &str = "network-empty";

/// Remote schemes the dialog accepts. `ssh` is GVfs' alias of `sftp`.
const SERVER_SCHEMES: [&str; 8] = ["smb", "sftp", "ftp", "ftps", "dav", "davs", "afp", "nfs"];

/// GVfs' browse roots: `network:///` and the SMB workgroup list `smb:///`.
pub(in crate::ui) fn is_network_root(location: &Location) -> bool {
    location.uri_value().is_some_and(|uri| {
        matches!(
            uri,
            "network:///" | "network://" | "network:" | "smb:///" | "smb://"
        )
    })
}

/// What GVfs network discovery can rely on in this session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::ui) struct Discovery {
    pub network_backend: bool,
    pub avahi: bool,
    pub wsdd: bool,
    pub smb_browsing: bool,
}

impl Discovery {
    pub(in crate::ui) fn probe() -> Self {
        let schemes = supported_schemes();
        Self {
            network_backend: schemes.iter().any(|scheme| scheme == "network"),
            avahi: avahi_available(),
            wsdd: glib::find_program_in_path("wsdd").is_some(),
            smb_browsing: smb_browsing_enabled(),
        }
    }

    /// Translated reasons why discovery may find nothing, most fundamental first.
    pub(in crate::ui) fn reasons(&self) -> Vec<String> {
        let tr = crate::i18n::tr;
        if !self.network_backend {
            return vec![tr(
                "GVfs network browsing is not available on this system (package gvfs-backends is missing).",
            )];
        }
        let mut reasons = Vec::new();
        if !self.avahi {
            reasons.push(tr(
                "The Avahi service (mDNS/DNS-SD) is not running, so computers that announce themselves via Zeroconf cannot be found.",
            ));
        }
        if !self.wsdd {
            reasons.push(tr(
                "The WS-Discovery helper “wsdd” is not installed, so Windows computers and NAS devices are not discovered.",
            ));
        }
        if !self.smb_browsing {
            reasons.push(tr(
                "Browsing the Windows network (SMB) is turned off in the system settings.",
            ));
        }
        if reasons.is_empty() {
            reasons.push(tr(
                "No computer or shared folder announced itself on the local network.",
            ));
        }
        reasons
    }
}

fn supported_schemes() -> Vec<String> {
    gio::Vfs::default()
        .supported_uri_schemes()
        .iter()
        .map(|scheme| scheme.to_ascii_lowercase())
        .collect()
}

fn avahi_available() -> bool {
    const NAME: &str = "org.freedesktop.Avahi";
    let Ok(bus) = gio::bus_get_sync(gio::BusType::System, gio::Cancellable::NONE) else {
        return false;
    };
    let call = |method: &str, arguments: Option<&glib::Variant>| {
        bus.call_sync(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            method,
            arguments,
            None,
            gio::DBusCallFlags::NONE,
            500,
            gio::Cancellable::NONE,
        )
        .ok()
    };
    let owned = call("NameHasOwner", Some(&(NAME,).to_variant()))
        .and_then(|reply| reply.get::<(bool,)>())
        .is_some_and(|(owned,)| owned);
    owned
        || call("ListActivatableNames", None)
            .and_then(|reply| reply.get::<(Vec<String>,)>())
            .is_some_and(|(names,)| names.iter().any(|name| name == NAME))
}

fn smb_browsing_enabled() -> bool {
    const SCHEMA: &str = "org.gnome.system.smb";
    let installed = gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(SCHEMA, true))
        .is_some_and(|schema| schema.has_key("display-mode"));
    installed && gio::Settings::new(SCHEMA).string("display-mode") != "disabled"
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::ui) enum AddressError {
    Empty,
    MissingScheme(String),
    Invalid,
    MissingHost,
    UnsupportedScheme(String),
    BackendMissing(String),
}

impl AddressError {
    pub(in crate::ui) fn message(&self) -> String {
        match self {
            Self::Empty => crate::i18n::tr("Enter the address of a server."),
            Self::MissingScheme(suggestion) => rust_i18n::t!(
                "The address needs a protocol, for example %{suggestion}",
                suggestion = suggestion
            )
            .into_owned(),
            Self::Invalid => crate::i18n::tr("That is not a valid server address."),
            Self::MissingHost => crate::i18n::tr("The address does not name a server."),
            Self::UnsupportedScheme(scheme) => rust_i18n::t!(
                "“%{scheme}” is not a network protocol. Use smb://, sftp://, ftp:// or dav://.",
                scheme = scheme
            )
            .into_owned(),
            Self::BackendMissing(address) => backend_unavailable_message(address),
        }
    }
}

/// Normalizes a typed server address: a Windows `\\server\share` path becomes
/// `smb://`, `ssh://` becomes `sftp://`. Credentials typed into the address are
/// kept here and later moved out of the URI by the browser's mount flow.
pub(in crate::ui) fn server_address(
    input: &str,
    supported: &[String],
) -> Result<String, AddressError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(AddressError::Empty);
    }
    let address = match input.strip_prefix("\\\\") {
        Some(unc) => format!("smb://{}", unc.replace('\\', "/")),
        None if !input.contains("://") => {
            return Err(AddressError::MissingScheme(format!(
                "smb://{}",
                input.trim_start_matches('/')
            )));
        }
        None => input.to_owned(),
    };
    let (scheme, rest) = address.split_once("://").ok_or(AddressError::Invalid)?;
    let scheme = match scheme.to_ascii_lowercase().as_str() {
        "ssh" => "sftp".to_owned(),
        other => other.to_owned(),
    };
    if !SERVER_SCHEMES.contains(&scheme.as_str()) {
        return Err(AddressError::UnsupportedScheme(scheme));
    }
    let address = format!("{scheme}://{rest}");
    let uri = glib::Uri::parse(
        &address,
        glib::UriFlags::HAS_PASSWORD | glib::UriFlags::ENCODED,
    )
    .map_err(|_| AddressError::Invalid)?;
    if uri.host().is_none_or(|host| host.is_empty()) {
        return Err(AddressError::MissingHost);
    }
    if !supported.contains(&scheme) {
        return Err(AddressError::BackendMissing(address));
    }
    Ok(address)
}

/// Theme icon names for GVfs network entries: the workgroup, a server root
/// (`smb://nas/`, a DNS-SD or WS-Discovery host) or an SMB share.
pub(in crate::ui) fn network_icon_names(location: &Location) -> Option<&'static [&'static str]> {
    let uri = location.uri_value()?;
    let (scheme, rest) = uri.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if matches!(scheme.as_str(), "dns-sd" | "wsdd") {
        return Some(&["network-server", "computer"]);
    }
    if !SERVER_SCHEMES.contains(&scheme.as_str()) {
        return None;
    }
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.rsplit('@').next().unwrap_or(host);
    let segments = path.split('/').filter(|part| !part.is_empty()).count();
    match (host.is_empty(), segments) {
        (true, 0) if scheme == "smb" => Some(&["network-workgroup", "network-server"]),
        (false, 0) => Some(&["network-server", "computer"]),
        (false, 1) if matches!(scheme.as_str(), "smb" | "afp") => {
            Some(&["folder-remote", "folder"])
        }
        _ => None,
    }
}

/// German-readable text for GVfs failures that only carry an English message
/// (the system has no GVfs translation and many backends use G_IO_ERROR_FAILED).
pub(in crate::ui) fn gvfs_failure_message(error: &glib::Error) -> Option<String> {
    let original = error.message();
    let message = original.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| message.contains(needle));
    let explanation = if has(&["connection refused"]) {
        "The host refused the connection. Check the service and port."
    } else if has(&["timed out", "timeout"]) {
        "The connection timed out. Check the host and firewall."
    } else if has(&[
        "no route to host",
        "network is unreachable",
        "host is unreachable",
    ]) {
        "That host is unreachable. Check your network connection."
    } else if has(&[
        "name or service not known",
        "temporary failure in name resolution",
        "could not resolve",
        "unknown host",
        "no address associated",
    ]) {
        "That host couldn’t be found. Check the address and DNS settings."
    } else if has(&["failed to retrieve share list"]) {
        "The server did not provide a list of shared folders. Enter the share name in the address, for example smb://server/share."
    } else if has(&["no such file or directory", "bad network name", "not found"]) {
        "The shared folder or path does not exist on this server. Check the address."
    } else if message.trim() == "connection failed"
        || has(&["connection reset", "connection closed"])
    {
        "The connection to the server failed. Check the address, the port and whether the service is running."
    } else {
        return None;
    };
    Some(format!(
        "{}\n\n{}",
        crate::i18n::tr(explanation),
        rust_i18n::t!(
            "Message from the system: %{message}",
            message = sanitize_failure_message(original)
        )
    ))
}

/// GVfs' SFTP host-key question, rebuilt in the UI language. GVfs words it
/// in English here because the system has no GVfs translation.
pub(in crate::ui) fn gvfs_question_text(message: &str) -> Option<String> {
    let between = |start: &str, end: &str| {
        let (_, rest) = message.split_once(start)?;
        rest.split_once(end)
            .map(|(value, _)| value.trim().to_owned())
    };
    let host = between("Verifying the identity of “", "” failed")?;
    let identity = between("The identity sent by the remote computer is “", "”")?;
    let first = rust_i18n::t!(
        "The identity of “%{host}” could not be verified. This is normal when you log in to a computer for the first time.",
        host = host
    );
    let second = if identity.trim_matches('?').is_empty() {
        crate::i18n::tr(
            "The system could not read the server’s key fingerprint, so it cannot be compared here.",
        )
    } else {
        rust_i18n::t!(
            "The computer sent the key fingerprint “%{fingerprint}”. Compare it with the fingerprint you know from the administrator.",
            fingerprint = identity
        )
        .into_owned()
    };
    Some(format!("{first}\n\n{second}"))
}

/// GVfs' SFTP and SMB password prompts in the UI language.
pub(in crate::ui) fn gvfs_prompt_text(message: &str) -> Option<String> {
    let after = |start: &str| message.split_once(start).map(|(_, rest)| rest);
    if let Some(rest) = after("Enter password for “") {
        let (user, rest) = rest.split_once("” on “")?;
        let (host, _) = rest.split_once('”')?;
        return Some(
            rust_i18n::t!(
                "Enter the password for “%{user}” on “%{host}”.",
                user = user,
                host = host
            )
            .into_owned(),
        );
    }
    if let Some(rest) = after("Password required for share ") {
        let (share, host) = rest.trim_end().split_once(" on ")?;
        return Some(
            rust_i18n::t!(
                "The shared folder “%{share}” on “%{host}” requires a password.",
                share = share,
                host = host
            )
            .into_owned(),
        );
    }
    let host = after("Password required for ")?.trim_end();
    Some(rust_i18n::t!("The server “%{host}” requires a password.", host = host).into_owned())
}

/// Labels GVfs gives the host-key question's choices; the reply stays the index.
pub(in crate::ui) fn gvfs_choice_label(choice: &str) -> String {
    match choice {
        "Log In Anyway" | "Cancel Login" => crate::i18n::tr(choice),
        _ => choice.to_owned(),
    }
}

/// Shows the page for an empty `network:///`: what was searched, why
/// nothing may have been found, and the way to connect manually.
pub(in crate::ui) fn show_network_empty(
    stack: &gtk::Stack,
    loading: &crate::ui::loading_skeleton::DelayedLoading,
) {
    let page = match stack.child_by_name(NETWORK_PAGE) {
        Some(page) => page,
        None => {
            let page = network_empty_page();
            stack.add_named(&page, Some(NETWORK_PAGE));
            page.upcast()
        }
    };
    if let Some(reasons) = page
        .last_child()
        .and_then(|button| button.prev_sibling())
        .and_downcast::<gtk::Label>()
    {
        reasons.set_text(&Discovery::probe().reasons().join("\n"));
    }
    loading.show(NETWORK_PAGE);
}

fn network_empty_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    page.add_css_class("directory-feedback");
    page.add_css_class("network-empty");
    page.set_halign(gtk::Align::Center);
    page.set_valign(gtk::Align::Center);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page.append(&network_image());
    let title = gtk::Label::new(Some(&crate::i18n::tr("No network devices found")));
    title.add_css_class("status-message");
    title.add_css_class("network-empty-title");
    page.append(&title);
    let reasons = gtk::Label::new(None);
    reasons.add_css_class("network-empty-reasons");
    reasons.set_wrap(true);
    reasons.set_max_width_chars(70);
    reasons.set_justify(gtk::Justification::Center);
    page.append(&reasons);
    let connect = gtk::Button::with_label(&crate::i18n::tr("Connect to Server…"));
    connect.add_css_class("network-connect-button");
    connect.set_halign(gtk::Align::Center);
    connect.set_margin_top(6);
    connect.set_action_name(Some(CONNECT_ACTION));
    page.append(&connect);
    page
}

fn network_image() -> gtk::Image {
    let themed = gtk::gdk::Display::default()
        .map(|display| gtk::IconTheme::for_display(&display))
        .and_then(|theme| {
            ["network-workgroup", "network-server"]
                .into_iter()
                .find(|name| theme.has_icon(name))
        });
    let image = match themed {
        Some(name) => gtk::Image::from_icon_name(name),
        None => crate::assets::chrome_icon(crate::assets::icons::NETWORK),
    };
    image.set_pixel_size(48);
    image
}

/// The classic "Connect to Server" dialog. `connect` receives the normalized
/// address and, when asked for, the bookmark name; it runs after the dialog
/// closed, so the browser's own prompts (password, host key) can follow.
pub(in crate::ui) fn show_connect_dialog(
    parent: &impl IsA<gtk::Widget>,
    connect: impl Fn(String, Option<String>) + 'static,
) {
    let Some(ModalHost {
        overlay,
        blurred_root,
    }) = ModalHost::blurred_for(parent)
    else {
        return;
    };
    let tr = crate::i18n::tr;
    let layout = modal_layout(
        crate::assets::icons::NETWORK,
        &tr("Connect to Server"),
        &tr("Open a shared folder on another computer"),
        &tr("Connect"),
    );
    layout.content.add_css_class("wide");
    layout.content.add_css_class("connect-server-dialog");

    let fields = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let address_group = gtk::Box::new(gtk::Orientation::Vertical, 5);
    address_group.append(&form_label(&tr("Server address")));
    let address = form_entry();
    address.set_placeholder_text(Some(&tr("smb://server/share")));
    address.set_input_purpose(gtk::InputPurpose::Url);
    address_group.append(&address);
    let error = form_error_label();
    error.set_wrap(true);
    address_group.append(&error);
    let examples = gtk::Label::new(Some(&dialog_text(&tr(
        "Examples: smb://192.0.2.10/share, smb://nas/share, sftp://user@server",
    ))));
    examples.add_css_class("dim-label");
    examples.add_css_class("connect-server-examples");
    examples.set_xalign(0.0);
    examples.set_wrap(true);
    address_group.append(&examples);
    fields.append(&address_group);

    let bookmark = gtk::CheckButton::with_label(&tr("Add bookmark"));
    let bookmark_name = form_entry();
    bookmark_name.set_placeholder_text(Some(&tr("Name, for example NAS")));
    bookmark_name.set_sensitive(false);
    {
        let bookmark_name = bookmark_name.clone();
        bookmark.connect_toggled(move |bookmark| {
            bookmark_name.set_sensitive(bookmark.is_active());
        });
    }
    let bookmark_group = gtk::Box::new(gtk::Orientation::Vertical, 5);
    bookmark_group.append(&bookmark);
    bookmark_group.append(&bookmark_name);
    fields.append(&bookmark_group);

    let note = gtk::Label::new(Some(&dialog_text(&tr(
        "User name and password are asked for when the server needs them. Passwords are only stored if you choose so.",
    ))));
    note.add_css_class("dim-label");
    note.set_xalign(0.0);
    note.set_wrap(true);
    fields.append(&note);
    layout.body.append(&fields);

    let typed_address = address.clone();
    let layer = modal_layer(
        &layout.content,
        &overlay,
        blurred_root.clone(),
        Some(std::rc::Rc::new(move || !typed_address.text().is_empty())),
    );
    overlay.add_overlay(&layer);

    for button in [layout.cancel.clone(), layout.close.clone()] {
        let layer = layer.clone();
        let overlay = overlay.clone();
        let root = blurred_root.clone();
        button.connect_clicked(move |_| dismiss_modal_layer(&layer, &overlay, root.as_ref()));
    }
    {
        let layer = layer.clone();
        let overlay = overlay.clone();
        let root = blurred_root.clone();
        close_on_escape(&layer.clone(), move || {
            dismiss_modal_layer(&layer, &overlay, root.as_ref());
        });
    }
    {
        let error = error.clone();
        address.connect_changed(move |_| error.set_visible(false));
    }
    let confirm_layer = layer.clone();
    let confirm_overlay = overlay.clone();
    let confirm_root = blurred_root;
    let confirm_address = address.clone();
    layout.confirm.connect_clicked(move |_| {
        let normalized = match server_address(&confirm_address.text(), &supported_schemes()) {
            Ok(normalized) => normalized,
            Err(problem) => {
                error.set_text(&problem.message());
                error.set_visible(true);
                confirm_address.grab_focus();
                return;
            }
        };
        let name = bookmark
            .is_active()
            .then(|| bookmark_name.text().trim().to_owned());
        dismiss_modal_layer(&confirm_layer, &confirm_overlay, confirm_root.as_ref());
        connect(normalized, name);
    });
    submit_on_enter(&layout.body, &layout.confirm);
    address.grab_focus();
}

impl BrowserView {
    /// Connects to a server address from the dialog. Mounting, credential and
    /// host-key prompts and failure reports are the typed-location flow's.
    /// Returns the credential-free location the browser is going to show.
    pub(in crate::ui) fn connect_to_server(&self, address: &str) -> Option<Location> {
        let state = &self.state;
        match state.open_typed_location(address, None) {
            Ok(TypedLocation::Navigating) => {
                state.location_stack.set_visible_child_name("breadcrumbs");
                state.browser.focus_active();
                crate::services::sanitize_uri_credentials(address)
                    .ok()
                    .map(|(sanitized, _)| Location::uri(sanitized))
            }
            Ok(TypedLocation::Mounting { sanitized }) => Some(Location::uri(sanitized)),
            Err(error) => {
                show_error_dialog(
                    &state.overlay,
                    &crate::i18n::tr("Unable to connect"),
                    &error.message(),
                );
                None
            }
        }
    }
}

impl BrowserView {
    /// Reload on a server location whose GVfs connection is gone mounts it
    /// again (with the usual prompts) instead of repeating "not mounted".
    pub(in crate::ui) fn reconnect_if_disconnected(&self) -> bool {
        let Some(location) = self.state.browser.active_location() else {
            return false;
        };
        let Some(uri) = location.uri_value() else {
            return false;
        };
        let scheme = uri
            .split_once("://")
            .map(|(scheme, _)| scheme.to_ascii_lowercase());
        if !scheme.is_some_and(|scheme| SERVER_SCHEMES.contains(&scheme.as_str())) {
            return false;
        }
        let disconnected = crate::adapters::gio_file_for_location(&location)
            .find_enclosing_mount(gio::Cancellable::NONE)
            .is_err_and(|error| error.matches(gio::IOErrorEnum::NotMounted));
        disconnected && self.connect_to_server(uri).is_some()
    }
}

/// Whether two locations name the same GIO file (`smb://nas/a` and `smb://nas/a/`).
pub(in crate::ui) fn same_location(left: &Location, right: &Location) -> bool {
    left == right
        || crate::adapters::gio_file_for_location(left)
            .equal(&crate::adapters::gio_file_for_location(right))
}
