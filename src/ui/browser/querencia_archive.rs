// SPDX-License-Identifier: MIT

//! Querencia: archive commands through Querencia Archiv. When its program is
//! in PATH or its launcher is installed, "Extract here" and "Compress…" run
//! `querencia-archiv`, which shows its own progress, conflict and error
//! windows; otherwise Files' built-in archive support stays.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};

use crate::model::FileEntry;
use crate::services::validate_basename;
use crate::ui::browser::ViewState;
use crate::ui::browser::entry::{entry_kind_summary, item_count_label};
use crate::ui::collection_edit::update_basename_validation;
use crate::ui::controls::{
    form_entry, form_error_label, form_label, segmented_control, set_form_field_error,
};
use crate::ui::modal::{show_error_dialog, submit_on_enter};

#[cfg(test)]
mod tests;

const PROGRAM: &str = "querencia-archiv";
const LAUNCHER: &str = "org.querencia.Archiv.desktop";
/// The MimeType list of Querencia Archiv 0.1.0's launcher, for an
/// installation that has only the program.
const ARCHIVE_TYPES: [&str; 14] = [
    "application/zip",
    "application/x-tar",
    "application/gzip",
    "application/x-xz",
    "application/x-bzip2",
    "application/zstd",
    "application/x-7z-compressed",
    "application/vnd.rar",
    "application/x-compressed-tar",
    "application/x-xz-compressed-tar",
    "application/x-bzip2-compressed-tar",
    "application/x-bzip-compressed-tar",
    "application/x-zstd-compressed-tar",
    "application/x-zip-compressed",
];
/// The formats `querencia-archiv --erstellen` writes, as label and extension.
const CREATE_FORMATS: [(&str, &str); 3] =
    [("ZIP", "zip"), ("TAR.GZ", "tar.gz"), ("TAR.XZ", "tar.xz")];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ArchiveTool {
    program: PathBuf,
    types: Vec<String>,
}

impl ArchiveTool {
    /// Querencia Archiv, looked up whenever it is needed, so installing or
    /// removing it takes effect without restarting Files.
    pub(super) fn find() -> Option<Self> {
        let launcher = gio_unix::DesktopAppInfo::new(LAUNCHER).and_then(|launcher| {
            let program = glib::find_program_in_path(launcher.executable())?;
            let types = launcher
                .supported_types()
                .iter()
                .map(ToString::to_string)
                .collect();
            Some((program, types))
        });
        Self::from_parts(glib::find_program_in_path(PROGRAM), launcher)
    }

    fn from_parts(
        in_path: Option<PathBuf>,
        launcher: Option<(PathBuf, Vec<String>)>,
    ) -> Option<Self> {
        let (launcher_program, launcher_types) = launcher.unzip();
        let program = in_path.or(launcher_program)?;
        let types = launcher_types
            .filter(|types| !types.is_empty())
            .unwrap_or_else(|| ARCHIVE_TYPES.map(str::to_owned).into());
        Some(Self { program, types })
    }

    /// Whether Archiv opens `entry`: a local file whose name gives one of the
    /// launcher's archive types exactly (an OpenDocument file is a zip
    /// subtype, not an archive to extract).
    pub(super) fn extracts(&self, entry: &FileEntry) -> bool {
        if !entry.is_file() || entry.location.native_path().is_none() {
            return false;
        }
        let (content_type, _) = gio::content_type_guess(Some(&entry.display_name), None);
        self.types
            .iter()
            .any(|archive_type| gio::content_type_equals(&content_type, archive_type))
    }

    fn extract_here_command(&self, archive: &Path) -> Command {
        let mut command = Command::new(&self.program);
        command.arg("--entpacken-hier").arg(archive);
        command
    }

    fn create_command(&self, archive: &Path, items: &[PathBuf]) -> Command {
        let mut command = Command::new(&self.program);
        command.arg("--erstellen").arg(archive).args(items);
        command
    }
}

/// The archive's file name for the typed `name`: one `.{extension}`, even
/// when the name already ends with it.
fn archive_file_name(name: &str, extension: &str) -> String {
    let base = name.strip_suffix(&format!(".{extension}")).unwrap_or(name);
    format!("{base}.{extension}")
}

impl ViewState {
    pub(super) fn extract_with_archive_tool(&self, tool: &ArchiveTool, entry: &FileEntry) {
        if let Some(archive) = entry.location.native_path() {
            self.run_archive_tool(tool.extract_here_command(archive));
        }
    }

    fn run_archive_tool(&self, command: Command) {
        if let Err(error) = crate::ui::terminal::spawn_reaped(command) {
            tracing::warn!(%error, "unable to start Querencia Archiv");
            show_error_dialog(
                &self.overlay,
                &crate::i18n::tr("Querencia Archiv could not be started"),
                &error.to_string(),
            );
        }
    }

    /// Asks for the archive's name and format (those Archiv writes, no
    /// password), then lets Archiv create it next to the first item.
    pub(super) fn show_archive_tool_compress_dialog(
        self: &Rc<Self>,
        tool: ArchiveTool,
        entries: Vec<FileEntry>,
    ) {
        let items: Vec<PathBuf> = entries
            .iter()
            .filter_map(|entry| entry.location.native_path().map(Path::to_path_buf))
            .collect();
        if items.is_empty() || items.len() != entries.len() {
            return;
        }
        let directory = items[0]
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(glib::home_dir);
        let default_name = if entries.len() == 1 {
            entries[0].display_name.clone()
        } else {
            "archive".to_owned()
        };
        let title = rust_i18n::t!(
            "Compress %{value1}",
            value1 = item_count_label(entries.len())
        )
        .into_owned();

        let name_entry = form_entry();
        name_entry.set_text(&default_name);
        name_entry.connect_changed(|field| {
            update_basename_validation(field);
        });
        let dirty_name = name_entry.clone();
        let (body, confirm, dismiss) = self.build_archive_modal(
            crate::assets::icons::PACKAGE_PLUS,
            &title,
            &entry_kind_summary(&entries),
            &crate::i18n::tr("Compress"),
            Some(Rc::new(move || dirty_name.text() != default_name)),
        );
        let name_error = form_error_label();
        body.append(&form_label(&crate::i18n::tr("Archive name")));
        body.append(&name_entry);
        body.append(&name_error);

        let labels = CREATE_FORMATS.map(|(label, _)| label);
        let (format_control, format_options) = segmented_control(&labels, 0);
        body.append(&form_label(&crate::i18n::tr("archive.format")));
        body.append(&format_control);

        let name_for_confirm = name_entry.clone();
        let state = Rc::downgrade(self);
        confirm.connect_clicked(move |_| {
            let extension = format_options
                .iter()
                .position(gtk::ToggleButton::is_active)
                .map_or(CREATE_FORMATS[0].1, |index| CREATE_FORMATS[index].1);
            let file_name = archive_file_name(name_for_confirm.text().trim(), extension);
            if let Err(message) = validate_basename(&file_name) {
                set_form_field_error(&name_for_confirm, &name_error, Some(message));
                name_for_confirm.grab_focus();
                return;
            }
            let archive = directory.join(&file_name);
            if archive.symlink_metadata().is_ok() {
                set_form_field_error(&name_for_confirm, &name_error, Some("File already exists"));
                name_for_confirm.grab_focus();
                return;
            }
            dismiss();
            if let Some(state) = state.upgrade() {
                state.run_archive_tool(tool.create_command(&archive, &items));
            }
        });
        submit_on_enter(&body, &confirm);
        name_entry.grab_focus();
    }
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
