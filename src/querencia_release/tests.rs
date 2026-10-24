// SPDX-License-Identifier: MIT

use std::fs;

use super::{Release, load, parse};

const ALPHA_2_1: &str = r#"NAME="Querencia Desktop"
VERSION="Alpha 2.1"
VERSION_ID="0.1.0~alpha2.1"
HOME_URL="https://querencialinux.org"
BUG_REPORT_URL="https://github.com/querencia-desktop/querencia/issues"
DOCUMENTATION_URL="https://querencialinux.org/de/docs/"
"#;

#[test]
fn the_release_file_supplies_version_and_links() {
    assert_eq!(
        parse(ALPHA_2_1),
        Release {
            version: "Alpha 2.1".to_owned(),
            home_url: "https://querencialinux.org".to_owned(),
            bug_report_url: "https://github.com/querencia-desktop/querencia/issues".to_owned(),
            documentation_url: "https://querencialinux.org/de/docs/".to_owned(),
        }
    );
}

#[test]
fn comments_unknown_keys_unquoted_values_and_escapes_are_read_like_os_release() {
    let release = parse(
        "# Kommentar\n\nVERSION_CODENAME=neu\nVERSION=Alpha\\ 3\nHOME_URL='https://example.org'\nBUG_REPORT_URL=\"https://example.org/\\\"bugs\\$\\\\\"\nkaputt\n",
    );
    assert_eq!(release.version, "Alpha\\ 3");
    assert_eq!(release.home_url, "https://example.org");
    assert_eq!(release.bug_report_url, "https://example.org/\"bugs$\\");
    assert_eq!(
        release.documentation_url,
        Release::default().documentation_url
    );
}

#[test]
fn missing_or_empty_keys_keep_the_defaults() {
    let release = parse("VERSION=\"\"\nHOME_URL=\"https://querencialinux.org/neu\"\n");
    let defaults = Release::default();
    assert_eq!(release.version, defaults.version);
    assert_eq!(release.home_url, "https://querencialinux.org/neu");
    assert_eq!(release.bug_report_url, defaults.bug_report_url);
}

#[test]
fn without_the_release_file_the_version_file_and_then_the_defaults_apply() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let release_file = directory.path().join("release");
    let version_file = directory.path().join("version");

    assert_eq!(load(&release_file, &version_file), Release::default());

    fs::write(&version_file, "  Alpha 2.1  \nzweite Zeile\n").expect("version file");
    let release = load(&release_file, &version_file);
    assert_eq!(release.version, "Alpha 2.1");
    assert_eq!(release.home_url, Release::default().home_url);

    fs::write(&release_file, "VERSION=\"Alpha 2.3\"\n").expect("release file");
    assert_eq!(load(&release_file, &version_file).version, "Alpha 2.3");
}

#[test]
fn online_help_opens_the_german_or_english_documentation() {
    let release = parse(ALPHA_2_1);
    assert_eq!(
        release.documentation_url_for("de"),
        "https://querencialinux.org/de/docs/"
    );
    for locale in ["en", "fr", "es", "it", "zh-CN"] {
        assert_eq!(
            release.documentation_url_for(locale),
            "https://querencialinux.org/en/docs/"
        );
    }
}
