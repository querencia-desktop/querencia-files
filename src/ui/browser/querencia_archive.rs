// SPDX-License-Identifier: MIT

//! Querencia: "Extract here" through Querencia Archiv. When its program is in
//! PATH or its launcher is installed, querencia-archiv extracts and shows its
//! own progress, conflict, password and error windows; otherwise, and for
//! "Compress" and "Extract to", Files' built-in archive support stays.

use std::path::{Path, PathBuf};
use std::process::Command;

use gtk::prelude::*;
use gtk::{gio, glib};

use crate::model::FileEntry;
use crate::ui::browser::ViewState;
use crate::ui::modal::show_error_dialog;

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

    /// Whether Archiv opens the entry: a local file whose name gives one of the
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
}
// SPDX-FileCopyrightText: 2026 Querencia Desktop contributors
