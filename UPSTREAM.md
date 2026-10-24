# Herkunft von Querencia Files (Fork von Strata)

## Upstream

| | |
|---|---|
| Projekt | Strata – „A fast, keyboard-first file manager for Linux“ |
| Adresse | https://github.com/lgse/strata (in `Cargo.toml`: `https://github.com/LGSE/strata`) |
| Lizenz | MIT (Datei `LICENSE`, unverändert) |
| Urheber | LGSE Ltd. und die Mitwirkenden von Strata (siehe Git-Geschichte; Drittlizenzen in `THIRD_PARTY_LICENSES.md`) |
| Basis-Commit | `2317e1af6b97a2c8d93cad8eea7990110c70fbae` (14.10.2026, „fix(mise): install cargo-watch from GitHub releases (#1647)“) |
| Upstream-Version an der Basis | 1.0.0 (`Cargo.toml`) |
| Eigener Zweig | `querencia` (Standardzweig dieses Repos); Spitze: QF-0r-1 (Tag `querencia/qf-0r-1`) und die Veröffentlichungs-Commits danach |
| Upstream-Zweig im Fork | `main`, unverändert gleich dem Basis-Commit |

Die Querencia-Änderungen liegen auf dem Basis-Commit (ein Merge zwischen
eigenen Zweigen, QF-0n; keine Upstream-Commits dazwischen). Die
Upstream-Geschichte ist unverändert (Hashes, Autoren, Commit-Texte).

## Warum ein Fork

Querencia Desktop will einen Dateimanager im klassischen GNOME-2-/Nautilus-2.30-Stil
(Menüleiste, Werkzeugleiste, Ortszeile, Symbolthema GNOME Human, klassische
Dateioperationen, Datenträger und Netzwerk wie in Nautilus 2). Strata liefert den
schnellen, sicher gekapselten Kern (Sandbox für Vorschauen, GVfs-Anbindung,
Papierkorb, Archive). Die Änderungen sind überwiegend Querencia-spezifisch (Optik,
Standardwerte); einzelne Fehlerbehebungen kommen für einen Upstream-Pull-Request
in Frage (Abschnitt „Zurück an den Upstream“).

## Änderungen von Querencia

Schritte QF-0b … QF-0r; ab QF-0f hat jeder Schritt ein Tag `querencia/qf-0<x>`.
QF-0a ist in QF-0b enthalten. Für die Veröffentlichung sind in diesen Commits
Testdaten und Beispiele mit Namen und Adressen aus einem privaten Netz durch neutrale
Werte ersetzt (`nas-test`, `PC-TEST`, `192.0.2.x` aus TEST-NET-1, Benutzer `user`). QF-0k fasst 0h–0j als Rückfallziel zusammen (kein Merge-Commit).

| # | Schritt (Tag) | Änderung | Commit | upstream angeboten? |
|---|---|---|---|---|
| 1 | QF-0b | Standardthema „Classic Light“, Symbolansicht als Standard, Fenstertitel „Querencia Files“ und eine klassische Fensteraufteilung (Menüleiste, Werkzeugleiste, Ortszeile, Statusleiste) über das neue Stylesheet `src/classic.css`; die F1-Kürzelübersicht lässt sich umschalten. | `64f8f30c` | nein (Querencia-spezifisch) |
| 2 | QF-0c | Dateiliste, Vorschau und Miniaturen holen Symbole aus dem installierten Symbolthema (`src/ui/theme_icons.rs`), damit Ordner und Dateitypen wie in GNOME 2 aussehen. | `bfee3f3a` | nein |
| 3 | QF-0d | Der Vorschau-Helfer meldet Dekodierfehler im Log und dekodiert Rastergrafiken zusätzlich mit dem Crate `image` (`src/sandbox_helper/raster.rs`), wenn gdk-pixbuf in der Sandbox scheitert. | `c5399aa1` | ja, möglich |
| 4 | QF-0e | Ein Klick zeigt keine Vorschau mehr (`single_click_previews = false`), wie in Nautilus 2. | `7dc3425c` | nein |
| 5 | QF-0f (`querencia/qf-0f`) | Visuelle Überarbeitung nach GNOME 2/Human: Symbolthema `gnome-human` (wenn installiert), Farbpalette „Querencia Klassisch“, kleine Symbole in der Werkzeugleiste, hellere Seitenleiste, klassische Dateiliste; `docs/qf-0f-visuelle-ueberarbeitung.md`. | `1e26244e` | nein |
| 6 | QF-0g (`querencia/qf-0g`) | Klassische Dateioperationen: Zwischenablage kompatibel zu Nautilus/Caja/Thunar (`x-special/gnome-copied-files`), Strg+R lädt neu, Strg+N öffnet ein Fenster, Menüs Datei/Bearbeiten/Ansicht mit Kürzeln; `docs/qf-w4-dateioperationen.md`. | `b5db7449` | teilweise (Zwischenablage-Format) |
| 7 | QF-0h (`querencia/qf-0h`) | Das Pfadvervollständigungs-Popup bleibt am Eingabefeld. | `c03d6059` | ja, möglich |
| 8 | QF-0i (`querencia/qf-0i`) | Kein Strata-„S“ mehr in der Titelleiste (Symbol aus der Ressourcendatei entfernt). | `1ccde2a2` | nein |
| 9 | QF-0j (`querencia/qf-0j`) | Die Auswahl in der Dateiliste hat eine dezente äußere Markierung (CSS). | `fe15f253` | nein |
| 10 | QF-0k (`querencia/qf-0k`) | Zusammenführung von QF-0h, 0i, 0j als Rückfallziel (ohne Merge-Commit; entfernt eine weitere Zeile aus `data/strata.gresource.xml`). | `368abdda` | – |
| 11 | QF-0l (`querencia/qf-0l`) | Spaltenansicht repariert, keine automatische Erstauswahl, Tooltip nennt die aktive Ansicht. | `4a0f5e6b` | ja, möglich (Spaltenansicht) |
| 12 | QF-0m (`querencia/qf-0m`) | Netzwerkbrowser: „Mit Server verbinden …“ (SFTP über GVfs), erklärende leere Netzwerkansicht, übersetzte GVfs-Fehler, Lesezeichen für Server; `src/ui/browser/network.rs`. | `15ce5ccd` (Seitenzweig) | nein |
| 13 | QF-0n (`querencia/qf-0n`) | Merge von QF-0l und QF-0m. | `9afdc888` | – |
| 14 | QF-0o | Datenträger im Alltag: Gerätesymbole je Typ (eigene SVGs `data/icons/scalable/devices/strata-querencia-*.svg`), Dialog „Der Datenträger wird noch benutzt“ beim Auswerfen, Netzwerkansicht nach Gerät; `docs/querencia-devices.md`. | `06ae6ce1` | nein |
| 15 | QF-0o | Netzlaufwerke über ein Zeilensymbol „Trennen“ lösen. | `b3910ec6` | nein |
| 16 | QF-0o (`querencia/qf-0o`) | Die Belegt-Frage wird nicht mehr vom Auswurf-Fortschritt verdeckt. | `4b90e29a` | nein |
| 17 | QF-0o-1 (`querencia/qf-0o-1`) | Die SFTP-Host-Key-Frage wird auch mit deutschem und anderssprachigem GVfs erkannt. | `e0d021cd` | nein (GVfs-Wortlaut) |
| 18 | QF-0o-2 | Fingerabdruck in der SFTP-Host-Key-Frage (eigener Abruf des Serverschlüssels, wenn GVfs keinen liefert). | `30519044` | ja, möglich |
| 19 | QF-0o-2 (`querencia/qf-0o-2-adaf7cec`) | Der Host-Key-Abruf schließt stdin mit leerem Puffer (Absturz behoben). | `6996b7ae` | mit 18 |
| 20 | QF-0o-2 | Gerade Anführungszeichen in eigenen Kommentaren. | `637ead82` | – |
| 21 | QF-0o-2 (`querencia/qf-0o-2`) | Der Host-Key-Abruf bietet nie Zugangsdaten an; Fehlschläge stehen im Log. | `f28be021` | mit 18 |
| 22 | QF-0o-2 | Begründete Ausnahmen für eine interne Veröffentlichungsprüfung (nur Kommentare; im öffentlichen Stand wieder entfernt). | `e0282294` | – |
| 23 | QF-0o-2-1 (`querencia/qf-0o-2-1`) | Lizenzvermerk der eigenen Dateien (SPDX, Copyright „Querencia Desktop contributors“). | `951d50c0` | – |
| 24 | QF-0o-2-2 (`querencia/qf-0o-2-2`) | Copyright-Zeile ans Dateiende, Lizenzdoku als `docs/querencia-lizenz.md`. | `3d9a0ce8` | – |
| 25 | QF-0o-3 (`querencia/qf-0o-3`) | Menüleiste und Menüeinträge schreiben dunkel auf den Akzentverlauf (Kontrast 5,6:1 bis 7,0:1). | `2a716552` | nein |
| 26 | QF-0p | Testdaten, Doku und ein Beispiel ohne Namen und Adressen aus einem privaten Netz (`nas-test`, `PC-TEST`, `192.0.2.x`, Benutzer `user`), wie die Ersetzung oben. | `e40ca122` | – |
| 27 | QF-0p | Das Bild auf der Seite „Über“ ist das Querencia-Programmsymbol statt des Strata-Logos (erste Fassung mit eingebetteter Kopie, siehe 32). | `4dbf1a58` | nein |
| 28 | QF-0p | Nach „Trotzdem anmelden“ in der SFTP-Host-Key-Frage gleicht Files `known_hosts` mit dem gezeigten Schlüssel ab und bricht bei einem anderen Schlüssel ab. | `8acfb61e` | ja, möglich |
| 29 | QF-0p | Die D-Bus-Namen von `org.freedesktop.FileManager1` werden ohne gio-Rückruf angefordert (kein Abbruch mehr beim Ende des Busses). | `5055adaa` | ja, möglich |
| 30 | QF-0p | „Hier entpacken“ und „Komprimieren …“ rufen Querencia Archiv auf, wenn es installiert ist (`src/ui/browser/querencia_archive.rs`, `docs/querencia-archiv.md`). | `947ff114` | nein |
| 31 | QF-0p | Kommentare im Archiv-Anschluss ohne Backticks und Ellipse (ai-clean). Bis hier reicht die erste Fassung von QF-0p; sie wurde verworfen, 32 und 33 ersetzen sie. | `5f192eb4` | – |
| 32 | QF-0p | Info-Dialog ohne eingebettetes Bild: fehlt das Programmsymbol im Symbolthema, zeigt er `system-file-manager`. | `9409a6ca` | nein |
| 33 | QF-0p | Nur „Hier entpacken“ geht über Querencia Archiv; „Komprimieren …“ und „Entpacken nach …“ bleiben beim eingebauten Archivmodul, der eigene Komprimieren-Dialog aus 30 entfällt. | `ee048eb8` | nein |
| 34 | QF-0p (`querencia/qf-0p`) | Files meldet das GTK-Farbschema nach dem Start einmal, damit ein neu gestartetes Files bei `prefer-dark` dunkel erscheint (QD-081). | `fe43a2a9` | ja, möglich |
| 35 | QF-0q (Tag des Erststands nicht übernommen) | Der Update-Prüfer von Strata ist stillgelegt; Querencia Files kommt über die Paketquelle. | `dd20017a` | nein |
| 36 | QF-0q-1 (`querencia/qf-0q-1`) | Paketquellen-Hinweis als reine Textzeile, Suchtest korrigiert. | `fa91f4a0` | nein |
| 37 | QF-0r (`querencia/qf-0r-1`) | Das Programm heißt überall „Querencia Files“; Strata steht nur noch als Herkunft auf der Seite „Über“ (mit Lizenztext); der Starter `data/io.github.lgse.Strata.desktop` heißt „Querencia Files“. | `da939d5b` | nein |
| 38 | – | Upstream-Dateien `docs/evidence/928/*` wiederhergestellt (siehe „Entfernt“). | `a1afd69e` | – |
| 39 | – | Veröffentlichungsvorlagen: Querencia-Abschnitt im README, diese Datei, `MARKE.md`, `SECURITY.md`, `.github/workflows/ci-querencia.yml`, Meldestelle. | `617e36c0` | – |
| 40 | – | Sechs Upstream-Workflows entfernt (siehe „Entfernt“). | `e530517c` | – |
| 41 | – | Doku: interne Bezüge entfernt, Installationshinweis ergänzt. | `af837e0e` | – |
| 42 | QF-0s (`querencia/qf-0s`) | Ziehen in den Papierkorb verschiebt, Info-Dialog mit Paketversion, Hilfe-Menü. | `06de6129` | nein |
| 43 | QF-0t | Versionsdatei, Dunkelmodus, App-ID `org.querencia.Files`, letzte Strata-Reste in Texten. | `fdcbfa2d` | nein |
| 44 | QF-0t (`querencia/qf-0t`) | Tests auf die neue `--version`-Zeile und die Dateiwähler-ID. | `637aef74` | nein |
| 45 | QF-0t-2 (`querencia/qf-0t-2`) | Kennung nach dem Zusammenführen. | `1f4ca2f3` | nein |
| 46 | QF-0t-3 (`querencia/qf-0t-3`) | Kennung QF-0t-3; Ausnahmeliste für die Upstream-Testdaten (`.clean-check-ignore`, im veröffentlichten Stand nicht enthalten). | `939cdf53` | nein |
| 47 | – | Quellstand bis QF-0t-3 mit den Veröffentlichungsvorlagen zusammengeführt. | `6728b84d` | – |
| 48 | – | Diese Tabelle und der README-Stand nachgeführt. | `c7d6ce0a` | – |
| 49 | – | README: Archiv-Hinweis, abgelöst durch Querencia Dateien (Alpha 3). | (dieser Commit) | – |

### Neue Dateien

Liste mit Lizenzangabe: [`docs/querencia-lizenz.md`](docs/querencia-lizenz.md)
(Stylesheet `src/classic.css`, Gerätesymbole, Netzwerk- und Gerätemodule,
Vorschau-Rückfall, Symbolthema-Anbindung, Beschreibungen unter `docs/`). Dazu
kommen die Veröffentlichungsdateien dieses Zweigs: diese Datei, `MARKE.md`,
`SECURITY.md`, `.github/workflows/ci-querencia.yml` und der Querencia-Abschnitt
am Anfang von `README.md`.

### Entfernt

Bewusst entfernt (Entscheidung des Besitzers, 21.10.2026): die Workflows
`.github/workflows/ci.yml`, `release.yml`, `publish-aur.yml`, `pr-policy.yml`,
`discord-triage.yml` und `packaging.yml`. Sie laufen auf Strata-Infrastruktur (Blacksmith-Runner,
AUR-Schlüssel, Discord) und würden in diesem Fork Strata bauen, veröffentlichen oder
Pull Requests nach Strata-Regeln bewerten. Die Querencia-CI ist `ci-querencia.yml`.
`install.sh`, `.github/release.yml` und `scripts/` bleiben: Tests des Upstreams nutzen
sie (Fork-Regel). Beim Nachführen des Upstreams diese Löschung beibehalten.

QF-0b hatte `docs/evidence/928/*` (README und 6 Bildschirmfotos aus dem
Upstream-Pull-Request #931) unbeabsichtigt gelöscht, weil es aus einem Archiv ohne
diese Dateien entstand; Commit 38 stellt sie byte-gleich wieder her.

Außerdem geändert (vollständig: `git diff --stat main..querencia`): Standardwerte in
`src/ui/preferences.rs`, Titel und Stylesheets in `src/ui/window.rs`, Palette in
`data/themes/catalog.toml`, neue Texte in allen elf Sprachen unter `data/locales/`,
Symbol in `data/strata.gresource.xml`.

## Was nicht verändert wurde

`LICENSE`, `THIRD_PARTY_LICENSES.md`, `data/licenses/*`, `Cargo.toml` (Name `strata`,
Autor „LGSE Ltd.“, Version 1.0.0, Feature `rar` als Standard), `CONTRIBUTING.md`, der
Strata-Text in `README.md` (unter dem Querencia-Abschnitt), die übrigen Workflows unter
`.github/` und die Strata-eigenen Agentendateien `CLAUDE.md`, `AGENTS.md`, `.cursor/`,
`skills-lock.json` sind Upstream-Bestand und bleiben unverändert.

## Lizenz der Querencia-Änderungen

Ebenfalls MIT, Copyright „2026 Querencia Desktop contributors“ für die von Querencia
angelegten Dateien und die Querencia-Zeilen in geänderten Dateien; Einzelheiten in [`docs/querencia-lizenz.md`](docs/querencia-lizenz.md). Diese Datei
ersetzt keine Urheberangabe des Upstreams.

**Poppler:** Die PDF-Vorschau linkt dynamisch gegen `libpoppler-glib` (Poppler,
GPL-2.0 oder GPL-3.0). Der Quelltext bleibt MIT; ein gegen Poppler gelinktes Binary
wird nach den Bedingungen der GPL weitergegeben (Querencia-Paket: Hinweis in
`debian/copyright`).

**UnRAR:** Das Feature `rar` (Crates `unrar`/`unrar_sys`, unfreie UnRAR-Lizenz) ist
Upstream-Standard und bleibt in `Cargo.toml` unverändert. Querencia-Pakete und die
Querencia-CI bauen mit `--no-default-features`, also ohne UnRAR.

## Name und Logo

Strata nennt in README, `CONTRIBUTING.md` und `LICENSE` keine Markenregel; das Logo
(`docs/assets/logos/strata-*.svg`, `data/brand/strata-logo-white.svg`,
`data/brand/tenxer-splash.png`) liegt im MIT-Repo. Querencia nutzt den Namen
„Strata“ nicht als eigenen Produktnamen („Querencia Files“); das Strata-„S“ ist in
der Titelleiste entfernt (QF-0i). Das Logo-Bild im Info-Dialog ist seit QF-0p durch das
Querencia-Symbol `org.querencia.Files` aus dem Symbolthema ersetzt (fehlt es, das
Themensymbol `system-file-manager`); der Fork bettet dafür kein Bild ein.
`data/brand/strata-logo-white.svg` bleibt als Upstream-Datei liegen und wird nicht
mehr angezeigt. Der Startbildschirm des „10x“-Modus (`data/brand/tenxer-splash.png`)
und der Strata-Text im README bleiben Upstream-Bestand. Querencia ist mit LGSE
Ltd. und Strata weder verbunden noch von ihnen unterstützt. Name und Logo
„Querencia“: [`MARKE.md`](MARKE.md).

## Upstream nachführen

```bash
git remote add upstream https://github.com/lgse/strata.git   # einmalig
git fetch upstream
git tag upstream/<version> upstream/main
git switch querencia
git merge upstream/main
# Konflikte lösen (meist src/ui/browser.rs, src/ui/window.rs, data/locales), dann:
cargo fmt --all --check
cargo clippy --all-targets --locked --all-features -- -D warnings
cargo test --locked --no-default-features
```

Danach Basis-Commit oben aktualisieren, Tag `querencia/<version>` setzen, isolierte
GUI-Prüfung.

**Workflows des Upstreams abschalten** (Fork und internes Repo: *Actions* →
Workflow → *Disable workflow*, oder Actions nur für `ci-querencia.yml` zulassen):
`sync-upstream-main.yml`, `e2e-images.yml` und `e2e-cache.yml` sind auf
Strata-Infrastruktur zugeschnitten und bleiben als Upstream-Bestand liegen; deaktivieren.
Die sechs übrigen Strata-Workflows sind entfernt (siehe „Entfernt“).

## Zurück an den Upstream

Noch nichts eingereicht. Kandidaten: QF-0d (Diagnose und Dekodier-Rückfall im
Vorschau-Helfer), QF-0h (Pfadvervollständigung), QF-0l (Spaltenansicht), QF-0g
(Zwischenablageformat), QF-0o-2 (Fingerabdruck in der SFTP-Host-Key-Frage).
