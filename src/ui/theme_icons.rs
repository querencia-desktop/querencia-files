// SPDX-License-Identifier: MIT

//! Desktop icon-theme icons for files, folders and places. GIO's shared MIME
//! database names the icons, the active GTK icon theme decides which of them
//! exist, and Strata's bundled icons stay the last resort, so a theme without
//! a matching icon never leaves a tile empty.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::Path,
};

use gtk::{gdk, gio, prelude::*};

use crate::{assets::icons, model::FileEntry};

#[cfg(test)]
mod tests;

/// Folder icon names, most specific first.
const FOLDER_NAMES: [&str; 2] = ["folder", "inode-directory"];

pub(crate) fn entry_icon_names(entry: &FileEntry) -> Vec<String> {
    if entry.is_broken_symbolic_link() {
        return Vec::new();
    }
    if entry.is_directory() {
        return folder_icon_names();
    }
    file_name_icon_names(&entry.display_name)
}

pub(crate) fn folder_icon_names() -> Vec<String> {
    FOLDER_NAMES.iter().map(|name| (*name).to_owned()).collect()
}

/// Names for a file known only by its name: GIO guesses the content type
/// from the MIME globs without touching the file.
pub(crate) fn file_name_icon_names(name: &str) -> Vec<String> {
    let (content_type, _uncertain) = gio::content_type_guess(Some(name), None);
    let mut names: Vec<String> = gio::content_type_get_icon(&content_type)
        .downcast::<gio::ThemedIcon>()
        .map(|icon| icon.names().iter().map(ToString::to_string).collect())
        .unwrap_or_default();
    if let Some(generic) = gio::content_type_get_generic_icon_name(&content_type)
        && !names.iter().any(|name| *name == generic)
    {
        names.push(generic.to_string());
    }
    names
}

pub(crate) fn path_icon_names(path: &Path, is_folder: bool) -> Vec<String> {
    if is_folder {
        return folder_icon_names();
    }
    path.file_name()
        .map(|name| file_name_icon_names(&name.to_string_lossy()))
        .unwrap_or_default()
}

/// Classic theme names for the sidebar's bundled place icons.
pub(crate) fn place_icon_names(bundled: &str) -> &'static [&'static str] {
    match bundled {
        icons::HOME => &["user-home"],
        icons::TRASH => &["user-trash"],
        icons::NETWORK => &["network-workgroup", "network-server"],
        icons::FOLDER => &["folder"],
        icons::CLOCK => &["document-open-recent"],
        icons::HARD_DRIVE => &["drive-harddisk"],
        _ => &[],
    }
}

/// Theme names for a sidebar place at `path`, most specific first: the home
/// folder and the XDG user directories get their classic GNOME icons
/// (`user-home`, `user-desktop`, `folder-documents`, …), then the names for
/// the place's bundled icon, then the plain folder.
pub(crate) fn place_path_icon_names(path: &Path, bundled: &str) -> Vec<String> {
    use gtk::glib::UserDirectory;
    let mut names = Vec::new();
    if path == gtk::glib::home_dir() {
        names.push("user-home");
    }
    for (directory, name) in [
        (UserDirectory::Desktop, "user-desktop"),
        (UserDirectory::Documents, "folder-documents"),
        (UserDirectory::Downloads, "folder-download"),
        (UserDirectory::Music, "folder-music"),
        (UserDirectory::Pictures, "folder-pictures"),
        (UserDirectory::PublicShare, "folder-publicshare"),
        (UserDirectory::Templates, "folder-templates"),
        (UserDirectory::Videos, "folder-videos"),
    ] {
        // XDG leaves unset directories at $HOME; that is the home folder.
        if gtk::glib::user_special_dir(directory).is_some_and(|dir| dir == path)
            && path != gtk::glib::home_dir()
        {
            names.push(name);
        }
    }
    names.extend(place_icon_names(bundled).iter().copied());
    names.extend(FOLDER_NAMES);
    let mut unique: Vec<String> = Vec::new();
    for name in names {
        if !unique.iter().any(|known| known == name) {
            unique.push(name.to_owned());
        }
    }
    unique
}

/// The first of `names` the active theme provides, trying the full-colour
/// names before their `-symbolic` variants.
pub(crate) fn resolve(names: &[String]) -> Option<String> {
    let display = gdk::Display::default()?;
    let theme = gtk::IconTheme::for_display(&display);
    names
        .iter()
        .find(|name| theme.has_icon(name))
        .cloned()
        .or_else(|| {
            names
                .iter()
                .map(|name| format!("{name}-symbolic"))
                .find(|name| theme.has_icon(name))
        })
}

thread_local! {
    static PAINTABLES: RefCell<HashMap<(String, i32, i32), gtk::IconPaintable>> =
        RefCell::new(HashMap::new());
    static WATCHING_THEME: Cell<bool> = const { Cell::new(false) };
}

/// A theme icon for `names` at `size` logical pixels, or `None` when the
/// theme has none of them. Paintables are shared per name and size so slots
/// rebinding the same entry keep their identity; a theme change drops them.
pub(crate) fn paintable(names: &[String], size: i32, scale: i32) -> Option<gdk::Paintable> {
    let name = resolve(names)?;
    let display = gdk::Display::default()?;
    let theme = gtk::IconTheme::for_display(&display);
    if !WATCHING_THEME.with(|watching| watching.replace(true)) {
        theme.connect_changed(|_| PAINTABLES.with(|cache| cache.borrow_mut().clear()));
    }
    let key = (name, size, scale);
    PAINTABLES.with(|cache| {
        if let Some(icon) = cache.borrow().get(&key) {
            return Some(icon.clone().upcast());
        }
        let icon = theme.lookup_icon(
            &key.0,
            &[],
            size,
            scale,
            gtk::TextDirection::Ltr,
            gtk::IconLookupFlags::empty(),
        );
        cache.borrow_mut().insert(key, icon.clone());
        Some(icon.upcast())
    })
}

/// A sidebar place icon from the theme, or Strata's bundled icon.
pub(crate) fn place_image(bundled: &str, pixel_size: i32) -> gtk::Image {
    let names: Vec<String> = place_icon_names(bundled)
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    match resolve(&names) {
        Some(name) => {
            let image = gtk::Image::from_icon_name(&name);
            image.set_pixel_size(pixel_size);
            image.add_css_class("theme-icon");
            image
        }
        None => crate::assets::primary_icon(bundled, pixel_size),
    }
}

/// GNOME 2's icon theme (GNOME Human: orange folders, inheriting the
/// classic GNOME icons). Querencia Desktop uses it, as the panel does since
/// HMP-04.
pub(crate) const CLASSIC_ICON_THEME: &str = "gnome-human";

/// The icon theme Querencia Files switches to: [`CLASSIC_ICON_THEME`] if the
/// user kept GTK's built-in default (`current` empty, Adwaita or hicolor)
/// and the theme is installed; `None` keeps the user's own choice.
pub(crate) fn classic_icon_theme(
    current: &str,
    installed: impl Fn(&str) -> bool,
) -> Option<&'static str> {
    let built_in_default = matches!(current, "" | "Adwaita" | "hicolor");
    (built_in_default && installed(CLASSIC_ICON_THEME)).then_some(CLASSIC_ICON_THEME)
}

/// Whether one of `data_dirs` has `icons/<name>/index.theme`.
pub(crate) fn icon_theme_installed(name: &str, data_dirs: &[std::path::PathBuf]) -> bool {
    data_dirs
        .iter()
        .any(|dir| dir.join("icons").join(name).join("index.theme").is_file())
}

/// Switches this process to the classic icon theme where
/// [`classic_icon_theme`] says so. Only the Querencia Files windows call it;
/// the portal file chooser keeps the desktop's theme.
pub(crate) fn use_classic_icon_theme() {
    let Some(settings) = gtk::Settings::default() else {
        return;
    };
    let current = settings
        .gtk_icon_theme_name()
        .map(|name| name.to_string())
        .unwrap_or_default();
    let mut data_dirs = vec![gtk::glib::user_data_dir()];
    data_dirs.extend(gtk::glib::system_data_dirs());
    if let Some(theme) = classic_icon_theme(&current, |name| icon_theme_installed(name, &data_dirs))
    {
        tracing::info!(from = %current, to = theme, "classic icon theme");
        settings.set_gtk_icon_theme_name(Some(theme));
    }
}
