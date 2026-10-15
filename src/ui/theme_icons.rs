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
        _ => &[],
    }
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
