// SPDX-License-Identifier: MIT

//! Querencia (QD-127): the Querencia Desktop release this installation
//! belongs to, read from the release file the `querencia-desktop` package
//! ships (os-release syntax). Without the file the built-in defaults apply.

use std::{fs, path::Path, sync::OnceLock};

const RELEASE_FILE: &str = "/usr/share/querencia/release";
const VERSION_FILE: &str = "/usr/share/querencia/version";

const DEFAULT_VERSION: &str = "Alpha 2.2";
const DEFAULT_HOME_URL: &str = "https://querencialinux.org";
const DEFAULT_BUG_REPORT_URL: &str =
    "https://github.com/querencia-desktop/querencia/issues/new/choose";
const DEFAULT_DOCUMENTATION_URL: &str = "https://querencialinux.org/de/docs/";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub home_url: String,
    pub bug_report_url: String,
    pub documentation_url: String,
}

impl Default for Release {
    fn default() -> Self {
        Self {
            version: DEFAULT_VERSION.to_owned(),
            home_url: DEFAULT_HOME_URL.to_owned(),
            bug_report_url: DEFAULT_BUG_REPORT_URL.to_owned(),
            documentation_url: DEFAULT_DOCUMENTATION_URL.to_owned(),
        }
    }
}

impl Release {
    /// The documentation exists in German and English; other languages get
    /// the English page.
    pub fn documentation_url_for(&self, locale: &str) -> String {
        if locale == "de" {
            self.documentation_url.clone()
        } else {
            self.documentation_url.replace("/de/docs", "/en/docs")
        }
    }
}

pub fn current() -> &'static Release {
    static RELEASE: OnceLock<Release> = OnceLock::new();
    RELEASE.get_or_init(|| load(Path::new(RELEASE_FILE), Path::new(VERSION_FILE)))
}

fn load(release_file: &Path, version_file: &Path) -> Release {
    if let Ok(contents) = fs::read_to_string(release_file) {
        return parse(&contents);
    }
    let mut release = Release::default();
    if let Some(version) = fs::read_to_string(version_file)
        .ok()
        .and_then(|contents| contents.lines().next().map(|line| line.trim().to_owned()))
        .filter(|line| !line.is_empty())
    {
        release.version = version;
    }
    release
}

/// Keys the file leaves out or leaves empty keep their defaults.
fn parse(contents: &str) -> Release {
    let mut release = Release::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = unquote(value.trim());
        if value.is_empty() {
            continue;
        }
        let field = match key.trim() {
            "VERSION" => &mut release.version,
            "HOME_URL" => &mut release.home_url,
            "BUG_REPORT_URL" => &mut release.bug_report_url,
            "DOCUMENTATION_URL" => &mut release.documentation_url,
            _ => continue,
        };
        *field = value;
    }
    release
}

/// os-release(5): optional single or double quotes; `\"`, `\\`, `` \` `` and
/// `\$` are the only escapes.
fn unquote(value: &str) -> String {
    let inner = ['"', '\'']
        .into_iter()
        .find_map(|quote| {
            value
                .strip_prefix(quote)
                .and_then(|rest| rest.strip_suffix(quote))
        })
        .unwrap_or(value);
    let mut unquoted = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next() {
                Some(escaped @ ('"' | '\\' | '`' | '$')) => unquoted.push(escaped),
                Some(other) => {
                    unquoted.push('\\');
                    unquoted.push(other);
                }
                None => unquoted.push('\\'),
            }
        } else {
            unquoted.push(character);
        }
    }
    unquoted
}

#[cfg(test)]
mod tests;
