// SPDX-License-Identifier: MIT

//! QF-0m: classic network browsing on top of GVfs. Discovery stays GVfs'
//! `network:///`; this adds the "Connect to Server" dialog, an explanatory
//! page when nothing was discovered, server/share icons and readable GVfs
//! failures. No protocol is implemented here: every connection is a GIO
//! mount through the browser's existing mount and authentication flow.

use gtk::prelude::*;
use gtk::{gio, glib};

use crate::assets::icons;
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

const SERVER_ICONS: &[&str] = &[icons::NETWORK_SERVER, "network-server", "computer"];
const WORKGROUP_ICONS: &[&str] = &[
    icons::NETWORK_WORKGROUP,
    "network-workgroup",
    "network-server",
];
const SHARE_ICONS: &[&str] = &[icons::NETWORK_SHARE, "folder-remote", "folder"];

/// Icon names for GVfs network entries: the workgroup, a server root
/// (`smb://nas/`, a DNS-SD or WS-Discovery host) or an SMB share. Querencia's
/// own icons come first, the classic theme names stay as fallback.
pub(in crate::ui) fn network_icon_names(location: &Location) -> Option<&'static [&'static str]> {
    let uri = location.uri_value()?;
    let (scheme, rest) = uri.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if matches!(scheme.as_str(), "dns-sd" | "wsdd") {
        return Some(SERVER_ICONS);
    }
    if !SERVER_SCHEMES.contains(&scheme.as_str()) {
        return None;
    }
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.rsplit('@').next().unwrap_or(host);
    let segments = path.split('/').filter(|part| !part.is_empty()).count();
    match (host.is_empty(), segments) {
        (true, 0) if scheme == "smb" => Some(WORKGROUP_ICONS),
        (false, 0) => Some(SERVER_ICONS),
        (false, 1) if matches!(scheme.as_str(), "smb" | "afp") => Some(SHARE_ICONS),
        _ => None,
    }
}

/// Querencia: names every discovery service a merged network entry came from.
pub(in crate::ui) fn network_sources_tooltip(location: &Location) -> Option<String> {
    use crate::adapters::DiscoveryService;
    let sources = crate::adapters::merged_network_sources(location)?;
    let lines: Vec<String> = sources
        .iter()
        .map(|source| match source.service {
            DiscoveryService::DnsSd => rust_i18n::t!(
                "Found via DNS-SD (Avahi): %{name} → %{target}",
                name = source.display_name,
                target = source.target
            ),
            DiscoveryService::WsDiscovery => rust_i18n::t!(
                "Found via WS-Discovery (wsdd): %{name} → %{target}",
                name = source.display_name,
                target = source.target
            ),
        })
        .map(|line| line.into_owned())
        .collect();
    Some(lines.join("\n"))
}

/// Readable text for GVfs failures that only carry the backend's English or
/// German message (many backends use G_IO_ERROR_FAILED).
pub(in crate::ui) fn gvfs_failure_message(error: &glib::Error) -> Option<String> {
    let original = error.message();
    let message = original.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|needle| message.contains(needle));
    let explanation = if has(&[
        "connection refused",
        "verbindungsaufbau abgelehnt",
        "vom server verweigert",
    ]) {
        "The host refused the connection. Check the service and port."
    } else if has(&[
        "timed out",
        "timeout",
        "wartezeit für die verbindung ist abgelaufen",
        "zeitüberschreitung",
    ]) {
        "The connection timed out. Check the host and firewall."
    } else if has(&[
        "no route to host",
        "network is unreachable",
        "host is unreachable",
        "keine route zum zielrechner",
        "netzwerk ist nicht erreichbar",
        "rechner ist nicht erreichbar",
    ]) {
        "That host is unreachable. Check your network connection."
    } else if has(&[
        "name or service not known",
        "temporary failure in name resolution",
        "could not resolve",
        "unknown host",
        "no address associated",
        "name oder der dienst ist nicht bekannt",
        "fehler bei der namensauflösung",
        "fehler beim auflösen",
        "gehört keine adresse",
    ]) {
        "That host couldn’t be found. Check the address and DNS settings."
    } else if has(&[
        "failed to retrieve share list",
        "empfangen der freigabenliste vom server ist gescheitert",
    ]) {
        "The server did not provide a list of shared folders. Enter the share name in the address, for example smb://server/share."
    } else if has(&[
        "no such file or directory",
        "bad network name",
        "not found",
        "nicht gefunden",
    ]) {
        "The shared folder or path does not exist on this server. Check the address."
    } else if matches!(
        message.trim(),
        "connection failed" | "verbindung fehlgeschlagen"
    ) || has(&[
        "connection reset",
        "connection closed",
        "vom kommunikationspartner zurückgesetzt",
        "von gegenstelle getrennt",
        "unerwartet getrennt",
    ]) {
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

/// Where GVfs' SFTP host-key question quotes the host and the fingerprint, in
/// each GVfs translation whose wording is known.
const GVFS_HOST_KEY_QUESTIONS: &[[(&str, &str); 2]] = &[
    [
        ("Verifying the identity of “", "” failed"),
        ("The identity sent by the remote computer is “", "”"),
    ],
    [
        ("Die Identität von »", "« konnte nicht überprüft werden"),
        (
            "Die vom entfernten Rechner übermittelte Identität lautet »",
            "«",
        ),
    ],
];

/// GVfs' placeholder when it could not read the server's key fingerprint.
const GVFS_UNKNOWN_FINGERPRINT: &str = "???";

/// Whether GVfs' question lacks the fingerprint. GVfs 1.60 expects OpenSSH's
/// older "key fingerprint is SHA256:...." and sends "???" for OpenSSH 10's
/// "key fingerprint is: SHA256:...".
pub(in crate::ui) fn gvfs_question_lacks_fingerprint(message: &str) -> bool {
    message.contains(GVFS_UNKNOWN_FINGERPRINT)
}

/// GVfs' SFTP host-key question, rebuilt in the UI language from the English
/// or German GVfs wording. Other wordings stay as GVfs sent them. Where GVfs
/// could not read the fingerprint, `probed` supplies it, or a note says it is
/// missing.
pub(in crate::ui) fn gvfs_question_text(message: &str, probed: Option<&HostKey>) -> Option<String> {
    let Some((host, identity)) = gvfs_question_parts(message) else {
        return gvfs_question_lacks_fingerprint(message).then(|| {
            format!(
                "{}\n\n{}",
                sanitize_failure_message(message),
                missing_fingerprint_note(probed)
            )
        });
    };
    let first = rust_i18n::t!(
        "The identity of “%{host}” could not be verified. This is normal when you log in to a computer for the first time.",
        host = host
    );
    let second = if identity.trim_matches('?').is_empty() {
        missing_fingerprint_note(probed)
    } else {
        rust_i18n::t!(
            "The computer sent the key fingerprint “%{fingerprint}”. Compare it with the fingerprint you know from the administrator.",
            fingerprint = identity
        )
        .into_owned()
    };
    Some(format!("{first}\n\n{second}"))
}

/// The host and the identity GVfs quotes in a known wording of its question.
fn gvfs_question_parts(message: &str) -> Option<(String, String)> {
    let between = |(start, end): (&str, &str)| {
        let (_, rest) = message.split_once(start)?;
        rest.split_once(end)
            .map(|(value, _)| value.trim().to_owned())
    };
    GVFS_HOST_KEY_QUESTIONS
        .iter()
        .find_map(|[host, identity]| Some((between(*host)?, between(*identity)?)))
}

/// The SHA256 fingerprint the host-key question showed: the one GVfs quoted,
/// else the one Querencia Files read itself.
pub(in crate::ui) fn shown_fingerprint(message: &str, probed: Option<&HostKey>) -> Option<String> {
    gvfs_question_parts(message)
        .map(|(_, identity)| identity)
        .filter(|identity| is_sha256_fingerprint(identity))
        .or_else(|| probed.map(|key| key.fingerprint.clone()))
}

fn is_sha256_fingerprint(value: &str) -> bool {
    value.strip_prefix("SHA256:").is_some_and(|digest| {
        digest.len() == 43
            && digest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
    })
}

fn missing_fingerprint_note(probed: Option<&HostKey>) -> String {
    match probed {
        Some(key) => rust_i18n::t!(
            "Querencia Files read the server’s key over a separate connection. %{key_type} key fingerprint:\n%{fingerprint}\nCompare it with the fingerprint you know from the administrator.",
            key_type = key.key_type,
            fingerprint = key.fingerprint
        )
        .into_owned(),
        None => crate::i18n::tr(
            "The system could not read the server’s key fingerprint, so it cannot be compared here.",
        ),
    }
}

/// A server host key as OpenSSH names and fingerprints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui) struct HostKey {
    pub(in crate::ui) key_type: String,
    pub(in crate::ui) fingerprint: String,
}

/// `ssh` arguments that read the SFTP server's host key the way GVfs' own
/// `ssh` does (same user configuration, aliases and algorithm order) and stop
/// at host-key verification: no known_hosts file is read or written. Every
/// authentication method is disabled, so no credentials are offered even if
/// the user configuration would accept the key (for example
/// NoHostAuthenticationForLocalhost or VerifyHostKeyDNS). `None` when the
/// location has no usable host.
pub(in crate::ui) fn host_key_probe_argv(location: &Location) -> Option<Vec<String>> {
    let target = ssh_target_argv(location)?;
    let mut argv: Vec<String> = [
        "ssh",
        "-v",
        "-oBatchMode=yes",
        "-oStrictHostKeyChecking=yes",
        "-oUserKnownHostsFile=/dev/null",
        "-oGlobalKnownHostsFile=/dev/null",
        "-oKnownHostsCommand=none",
        "-oUpdateHostKeys=no",
        "-oCheckHostIP=no",
        "-oFingerprintHash=sha256",
        "-oVisualHostKey=no",
        "-oConnectTimeout=5",
        "-oControlMaster=no",
        "-oControlPath=none",
        "-oForwardX11=no",
        "-oForwardAgent=no",
        "-oPermitLocalCommand=no",
        "-oClearAllForwardings=yes",
        "-oNoHostAuthenticationForLocalhost=no",
        "-oVerifyHostKeyDNS=no",
        "-oPubkeyAuthentication=no",
        "-oPasswordAuthentication=no",
        "-oKbdInteractiveAuthentication=no",
        "-oHostbasedAuthentication=no",
        "-oGSSAPIAuthentication=no",
    ]
    .map(str::to_owned)
    .into();
    argv.extend(target);
    argv.push("true".to_owned());
    Some(argv)
}

/// `[-p port] [-l user] -- host` of an SFTP location, or `None` when it has
/// no usable host.
fn ssh_target_argv(location: &Location) -> Option<Vec<String>> {
    if location.backend_name() != "sftp" && location.backend_name() != "ssh" {
        return None;
    }
    let uri = glib::Uri::parse(location.uri_value()?, glib::UriFlags::HAS_PASSWORD).ok()?;
    let usable = |value: &str| {
        !value.is_empty()
            && !value.starts_with('-')
            && !value.chars().any(|c| c.is_whitespace() || c.is_control())
    };
    let host = uri.host().filter(|host| usable(host))?;
    let mut argv = Vec::new();
    if uri.port() > 0 {
        argv.extend(["-p".to_owned(), uri.port().to_string()]);
    }
    if let Some(user) = uri.user().filter(|user| usable(user)) {
        argv.extend(["-l".to_owned(), user.to_string()]);
    }
    argv.extend(["--".to_owned(), host.to_string()]);
    Some(argv)
}

/// The host key from `ssh -v`'s "Server host key: <name> SHA256:<base64>".
pub(in crate::ui) fn host_key_from_ssh_log(log: &str) -> Option<HostKey> {
    let (name, fingerprint) = log.lines().find_map(|line| {
        let (_, rest) = line.split_once("Server host key: ")?;
        let mut parts = rest.split_whitespace();
        Some((parts.next()?, parts.next()?))
    })?;
    if !is_sha256_fingerprint(fingerprint) {
        return None;
    }
    let key_type = match name {
        "ssh-ed25519" => "ED25519",
        "sk-ssh-ed25519@openssh.com" => "ED25519-SK",
        "sk-ecdsa-sha2-nistp256@openssh.com" => "ECDSA-SK",
        "ssh-rsa" | "rsa-sha2-256" | "rsa-sha2-512" => "RSA",
        "ssh-dss" => "DSA",
        name if name.starts_with("ecdsa-sha2-") => "ECDSA",
        _ => return None,
    };
    Some(HostKey {
        key_type: key_type.to_owned(),
        fingerprint: fingerprint.to_owned(),
    })
}

const HOST_KEY_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// Runs the host-key probe; `None` when `ssh` is missing, times out or
/// reports no plain host key (for example a host certificate).
pub(in crate::ui) async fn probe_host_key(argv: Vec<String>) -> Option<HostKey> {
    let argv: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    let process = match gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDIN_PIPE
            | gio::SubprocessFlags::STDOUT_SILENCE
            | gio::SubprocessFlags::STDERR_PIPE,
    ) {
        Ok(process) => process,
        Err(error) => {
            tracing::info!(%error, "host-key probe could not start ssh");
            return None;
        }
    };
    let no_input = glib::Bytes::from_static(b"");
    let key = match glib::future_with_timeout(
        HOST_KEY_PROBE_TIMEOUT,
        process.communicate_future(Some(&no_input)),
    )
    .await
    {
        Ok(Ok((_, Some(log)))) => host_key_from_ssh_log(&String::from_utf8_lossy(&log)),
        Ok(Ok((_, None))) => None,
        Ok(Err(error)) => {
            tracing::info!(%error, "host-key probe failed");
            return None;
        }
        Err(_) => {
            process.force_exit();
            tracing::info!(
                timeout_s = HOST_KEY_PROBE_TIMEOUT.as_secs(),
                "host-key probe timed out"
            );
            return None;
        }
    };
    if key.is_none() {
        tracing::info!("host-key probe found no plain host key in the ssh log");
    }
    key
}

/// `ssh -G` for an SFTP location: resolves the user configuration (aliases,
/// `HostKeyAlias`, `UserKnownHostsFile`) without connecting.
pub(in crate::ui) fn ssh_config_argv(location: &Location) -> Option<Vec<String>> {
    let mut argv = vec!["ssh".to_owned(), "-G".to_owned()];
    argv.extend(ssh_target_argv(location)?);
    Some(argv)
}

/// Where `ssh` saves a host's key: its name in known_hosts and the user
/// known-hosts files, as `ssh -G` printed them.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::ui) struct KnownHostsTarget {
    pub(in crate::ui) name: String,
    pub(in crate::ui) files: Vec<String>,
}

pub(in crate::ui) fn known_hosts_target(config: &str) -> Option<KnownHostsTarget> {
    let value = |key: &str| {
        config.lines().find_map(|line| {
            let (name, value) = line.split_once(' ')?;
            (name == key).then(|| value.trim())
        })
    };
    let name = match value("hostkeyalias").filter(|alias| !alias.is_empty() && *alias != "none") {
        Some(alias) => alias.to_owned(),
        None => {
            let host = value("hostname").filter(|host| !host.is_empty())?;
            match value("port") {
                Some(port) if port != "22" => format!("[{host}]:{port}"),
                _ => host.to_owned(),
            }
        }
    };
    let files = value("userknownhostsfile")?
        .split_whitespace()
        .filter(|file| *file != "none")
        .map(str::to_owned)
        .collect();
    Some(KnownHostsTarget { name, files })
}

pub(in crate::ui) fn known_hosts_lookup_argv(name: &str, file: &str) -> Vec<String> {
    ["ssh-keygen", "-l", "-F", name, "-f", file]
        .map(str::to_owned)
        .into()
}

/// The fingerprints in `ssh-keygen -l -F`'s listing, without marked
/// (`@revoked`, `@cert-authority`) entries.
pub(in crate::ui) fn known_hosts_fingerprints(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with('@'))
        .filter_map(|line| line.split_whitespace().last())
        .filter(|fingerprint| is_sha256_fingerprint(fingerprint))
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::ui) enum KnownHostsCheck {
    Matches,
    NotSaved,
    Differs { name: String, file: String },
}

/// Compares the fingerprint the question showed with what known_hosts holds
/// for the host after the user accepted it; `saved` lists each file's
/// fingerprints in ssh's order.
pub(in crate::ui) fn compare_known_hosts(
    shown: &str,
    name: &str,
    saved: &[(String, Vec<String>)],
) -> KnownHostsCheck {
    if saved
        .iter()
        .any(|(_, fingerprints)| fingerprints.iter().any(|saved| saved == shown))
    {
        return KnownHostsCheck::Matches;
    }
    saved
        .iter()
        .find(|(_, fingerprints)| !fingerprints.is_empty())
        .map_or(KnownHostsCheck::NotSaved, |(file, _)| {
            KnownHostsCheck::Differs {
                name: name.to_owned(),
                file: file.clone(),
            }
        })
}

const KNOWN_HOSTS_CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

async fn command_output(argv: &[String]) -> Option<String> {
    let argv: Vec<&std::ffi::OsStr> = argv.iter().map(std::ffi::OsStr::new).collect();
    let process = gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDIN_PIPE
            | gio::SubprocessFlags::STDOUT_PIPE
            | gio::SubprocessFlags::STDERR_SILENCE,
    )
    .inspect_err(|error| tracing::info!(%error, "known_hosts check could not start a command"))
    .ok()?;
    let no_input = glib::Bytes::from_static(b"");
    match glib::future_with_timeout(
        KNOWN_HOSTS_CHECK_TIMEOUT,
        process.communicate_future(Some(&no_input)),
    )
    .await
    {
        Ok(Ok((Some(output), _))) => Some(String::from_utf8_lossy(&output).into_owned()),
        Ok(Ok((None, _))) => Some(String::new()),
        Ok(Err(error)) => {
            tracing::info!(%error, "known_hosts check command failed");
            None
        }
        Err(_) => {
            process.force_exit();
            tracing::info!("known_hosts check command timed out");
            None
        }
    }
}

/// After the user accepted an SFTP host key: does the entry `ssh` saved match
/// the fingerprint the question showed? The key is read from known_hosts
/// because GVfs' login is a different connection than the probe.
pub(in crate::ui) async fn check_known_hosts(location: Location, shown: String) -> KnownHostsCheck {
    let Some(argv) = ssh_config_argv(&location) else {
        return KnownHostsCheck::NotSaved;
    };
    let Some(target) = command_output(&argv)
        .await
        .as_deref()
        .and_then(known_hosts_target)
    else {
        tracing::info!("known_hosts check could not resolve the ssh configuration");
        return KnownHostsCheck::NotSaved;
    };
    let mut saved = Vec::new();
    for file in target.files {
        if !std::path::Path::new(&file).is_file() {
            continue;
        }
        let listing = command_output(&known_hosts_lookup_argv(&target.name, &file)).await;
        saved.push((
            file,
            known_hosts_fingerprints(listing.as_deref().unwrap_or("")),
        ));
    }
    let check = compare_known_hosts(&shown, &target.name, &saved);
    match &check {
        KnownHostsCheck::Matches => {
            tracing::info!("known_hosts entry matches the accepted host key")
        }
        KnownHostsCheck::NotSaved => {
            tracing::info!("known_hosts holds no entry for the accepted host key")
        }
        KnownHostsCheck::Differs { .. } => {
            tracing::warn!("known_hosts entry differs from the accepted host key")
        }
    }
    check
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, glib::ErrorDomain)]
#[error_domain(name = "querencia-files-known-hosts")]
pub(in crate::ui) enum KnownHostsError {
    Differs,
}

/// The mount failure for a known_hosts entry that differs from the shown
/// fingerprint, with the command that removes the entry.
pub(in crate::ui) fn known_hosts_mismatch_error(
    shown: &str,
    name: &str,
    file: &str,
) -> glib::Error {
    let command = format!("ssh-keygen -R {} -f {}", shell_word(name), shell_word(file));
    glib::Error::new(
        KnownHostsError::Differs,
        &rust_i18n::t!(
            "Querencia Files cancelled the connection: during login the server presented a different key than the fingerprint shown before (%{fingerprint}). ssh has saved that other key. Remove it with “%{command}” and confirm the fingerprint with the server administrator before you connect again.",
            fingerprint = shown,
            command = command
        ),
    )
}

fn shell_word(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:@%+=,".contains(c))
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

/// GVfs' SFTP and SMB password prompts in the UI language.
pub(in crate::ui) fn gvfs_prompt_text(message: &str) -> Option<String> {
    let after = |start: &str| message.split_once(start).map(|(_, rest)| rest);
    if let Some(rest) = after("A passphrase is needed to access encrypted data on “") {
        let (device, _) = rest.split_once('”')?;
        return Some(
            rust_i18n::t!(
                "Enter the passphrase to unlock the encrypted data on “%{device}”.",
                device = device
            )
            .into_owned(),
        );
    }
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

/// Labels GVfs gives the host-key question's choices in English and German,
/// with the English message they show in the UI language.
const GVFS_CHOICE_LABELS: &[(&str, &str)] = &[
    ("Log In Anyway", "Log In Anyway"),
    ("Cancel Login", "Cancel Login"),
    ("Dennoch anmelden", "Log In Anyway"),
    ("Anmelden abbrechen", "Cancel Login"),
];

pub(in crate::ui) fn gvfs_choice_label(choice: &str) -> String {
    GVFS_CHOICE_LABELS
        .iter()
        .find(|(gvfs, _)| *gvfs == choice)
        .map_or_else(
            || choice.to_owned(),
            |(_, message)| crate::i18n::tr(message),
        )
}

fn is_rejection_choice(choice: &str) -> bool {
    let choice = choice.to_lowercase();
    let english = [
        "cancel",
        "reject",
        "deny",
        "disconnect",
        "abort",
        "do not",
        "don't",
        "no",
    ]
    .iter()
    .any(|word| choice == *word || choice.starts_with(&format!("{word} ")));
    let german = choice.split_whitespace().any(|word| {
        matches!(
            word,
            "abbrechen" | "nein" | "ablehnen" | "trennen" | "nicht"
        )
    });
    english || german
}

/// A GVfs question's choice as the dialog shows it; `index` is the reply.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::ui) struct GvfsChoice {
    pub(in crate::ui) index: usize,
    pub(in crate::ui) label: String,
    pub(in crate::ui) rejects: bool,
}

/// The choices in display order, safe choices first. GVfs lists the safe
/// choice last (“Cancel Login”, “No”, “Cancel”), so when no label is
/// recognized, as in other languages, that position marks it.
pub(in crate::ui) fn gvfs_choices(choices: &[String]) -> Vec<GvfsChoice> {
    let recognized: Vec<bool> = choices
        .iter()
        .map(|choice| is_rejection_choice(choice))
        .collect();
    let by_position = choices.len() > 1 && !recognized.contains(&true);
    let mut ordered: Vec<GvfsChoice> = choices
        .iter()
        .zip(recognized)
        .enumerate()
        .map(|(index, (choice, recognized))| GvfsChoice {
            index,
            label: gvfs_choice_label(choice),
            rejects: recognized || (by_position && index + 1 == choices.len()),
        })
        .collect();
    ordered.sort_by_key(|choice| !choice.rejects);
    ordered
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
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
