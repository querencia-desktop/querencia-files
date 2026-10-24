// SPDX-License-Identifier: MIT

use std::{collections::BTreeSet, path::Path};

const LOCALES: [&str; 11] = [
    "en", "fr", "de", "es", "ja", "pt-BR", "ko", "vi", "it", "ru", "zh-CN",
];

fn placeholders(text: &str) -> BTreeSet<&str> {
    text.split("%{")
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name))
        .collect()
}

fn catalog(path: &Path) -> serde_json::Map<String, serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(path).expect("read catalog"))
        .expect("valid catalog JSON")
        .as_object()
        .expect("catalog object")
        .clone()
}

fn is_context_key(key: &str) -> bool {
    key.contains('.')
        && key.split('.').all(|part| {
            !part.is_empty()
                && part.chars().all(|character| {
                    character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
                })
        })
}

#[test]
fn english_context_keys_have_english_text() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/locales");
    let mut checked = 0;
    for folder in [&root, &root.join("messages")] {
        for (key, value) in catalog(&folder.join("en.json")) {
            if is_context_key(&key) {
                checked += 1;
                assert_ne!(value.as_str(), Some(key.as_str()), "{}", folder.display());
            }
        }
    }
    assert!(checked > 0);
}

#[test]
fn catalogs_cover_all_languages_and_preserve_interpolation() {
    assert_eq!(
        rust_i18n::available_locales!()
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<_>>(),
        LOCALES.into_iter().map(String::from).collect()
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/locales");
    for folder in [&root, &root.join("messages")] {
        let english = catalog(&folder.join("en.json"));
        assert!(!english.is_empty());
        for locale in LOCALES {
            let translated = catalog(&folder.join(format!("{locale}.json")));
            assert_eq!(
                translated.keys().collect::<Vec<_>>(),
                english.keys().collect::<Vec<_>>(),
                "{}: {locale}",
                folder.display()
            );
            for (key, original) in &english {
                let value = translated[key].as_str().expect("translated string");
                assert!(!value.trim().is_empty(), "{locale}: {key}");
                assert_eq!(
                    placeholders(value),
                    placeholders(original.as_str().expect("English message")),
                    "{locale}: {key}"
                );
            }
        }
    }
    for file in ["counts.json", "dates.json"] {
        for (key, translations) in catalog(&root.join(file)) {
            if key == "_version" {
                continue;
            }
            let translations = translations.as_object().expect("multilingual message");
            assert_eq!(
                translations
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                LOCALES.into_iter().collect(),
                "{file}: {key}"
            );
            let english = translations["en"].as_str().expect("English message");
            for locale in LOCALES {
                let value = translations[locale].as_str().expect("translated message");
                assert!(!value.trim().is_empty(), "{locale}: {key}");
                assert_eq!(
                    placeholders(value),
                    placeholders(english),
                    "{locale}: {key}"
                );
            }
        }
    }
}

/// Querencia (QF-0r): what the catalogs show names the program "Querencia
/// Files"; the name Strata appears only in the origin notice (lowercase
/// "strata" stays in real paths and in the search keywords of the About page).
#[test]
fn querencia_catalog_values_name_strata_only_as_origin() {
    const ORIGIN_KEYS: [&str; 3] = [
        "Based on Strata by LGSE Ltd. (MIT)",
        "settings_keywords.version",
        "Querencia Files %{value1}\n%{value2}\nBuild: %{value8}\nCommit: %{value3}\nToolkit: %{value4}\nAuthor: %{value5}\nLicense: MIT\nBased on %{value6} (%{value7}, MIT)",
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/locales");
    let mut checked = 0;
    for folder in [&root, &root.join("messages")] {
        for locale in LOCALES {
            for (key, value) in catalog(&folder.join(format!("{locale}.json"))) {
                let value = value.as_str().expect("translated string");
                checked += 1;
                assert!(!value.contains("lgse/strata"), "{locale}: {key}");
                assert!(!value.contains("stratafiles"), "{locale}: {key}");
                if value.contains("Strata") {
                    assert!(
                        ORIGIN_KEYS.contains(&key.as_str()),
                        "{locale}: {key}: {value}"
                    );
                }
            }
        }
    }
    assert!(checked > 0);
}

/// Querencia (QF-0s): hints shown on every system name Ubuntu, not Arch or
/// Omarchy; the keys keep Strata's English text.
#[test]
fn querencia_media_and_portal_hints_do_not_name_arch_or_omarchy() {
    const KEYS: [&str; 2] = [
        "On Arch or Omarchy, install the required GStreamer plugins, then restart Strata.",
        "Use Strata for Open and Save dialogs, opening folders, and Reveal in File Manager. On Omarchy, this also sets file-manager keyboard shortcuts. Close open file dialogs before changing setup.",
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/locales");
    for locale in LOCALES {
        let values = catalog(&root.join(format!("{locale}.json")));
        for key in KEYS {
            let value = values[key].as_str().expect("translated string");
            assert!(
                !value.contains("Arch") && !value.contains("Omarchy"),
                "{locale}: {value}"
            );
        }
        let media = values[KEYS[0]].as_str().expect("translated string");
        assert!(media.contains("Ubuntu"), "{locale}: {media}");
    }
}
