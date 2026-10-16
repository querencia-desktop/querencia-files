# QF-0f – Visuelle Überarbeitung von Querencia Files

Ziel: Querencia Files soll wie ein klassischer GNOME-2-/Nautilus-2.30-
Dateimanager mit GNOME-Human-Optik aussehen. Funktion und Struktur bleiben
unverändert: Menüleiste, Werkzeugleiste, Ortszeile, Seitenleiste,
Dateiliste und Statusleiste. Der Besitzer sagte dazu: „Eigentlich ist er
super umgesetzt, aber visuell finde ich ihn noch nicht perfekt.“

## Ursachen der kühlen Wirkung (vorher)

| Befund | Ursache |
|---|---|
| hellblaue Ordner- und Dateisymbole | GTK-Standardthema **Adwaita** (`gsettings icon-theme 'Adwaita'`). Die Dateiliste fragt das Thema über GIO-MIME-Namen ab (`theme_icons.rs`), das Thema lieferte also Adwaita-Ordner. |
| blaue Auswahl, blaue Pfad- und Kopfzeilen | Standardthema „Classic Light“ (Base16) mit Akzent `#6A9FB5` |
| graue Seitenleiste | Fläche `#E0E0E0` desselben Themas |
| eingefärbte Umriss-Symbole bei „Persönlicher Ordner“ und „Desktop“ | Angeheftete Orte ersetzten das Themensymbol immer durch Strata-eigene, mit dem Akzent eingefärbte Symbole (`customize_sidebar_folder_icon`), auch ohne Benutzeranpassung |
| dominante Werkzeugleiste | 24-px-Symbole (GTK-„large toolbar“) |

## Änderungen

1. **Symbolthema GNOME Human:**
   - `theme_icons::use_classic_icon_theme()` schaltet den Prozess auf `gnome-human` um, aber nur, wenn
     - der Benutzer GTKs eingebauten Standard behalten hat (`""`, `Adwaita`, `hicolor`) und
     - das Thema installiert ist.
   - Dieselbe Regel nutzt das Panel seit HMP-04.
   - `gnome-human` erbt von `gnome-colors-common` und `gnome`, den klassischen GNOME-2-Symbolen; Adwaita wird nicht gemischt.
   - Aufruf nur beim Öffnen von Dateimanager-Fenstern; der Portal-Dateiwähler bleibt unberührt.
2. **Farbpalette „Querencia Klassisch“** (`data/themes/catalog.toml`, ID `querencia-classic`, neuer Standard):
   - Ambiance-Werte: Basis `#ffffff`, Fläche `#f2f1f0`, Text `#3c3c3c`, Auswahl `#f07746`;
   - Rahmen `#c7c2be` und Grautöne wie in den Panelmenüs (HMP-05);
   - Syntaxfarben aus der Tango-Palette.
   - Kontrast Text/Hintergrund 11,0:1, Text/Fläche 9,8:1.
   - Benutzer mit anderem Thema behalten es; ein unbekanntes gespeichertes Thema fällt auf dieses zurück.
3. **Werkzeugleiste:**
   - 16-px-Symbole (GTK-2-„small toolbar“), Knöpfe 22 px hoch;
   - Überfahren und Drücken neutral (heller Verlauf, Rahmen, beim Drücken eingedrückt), kein Akzent.
   - Zwei Varianten über `QUERENCIA_FILES_TOOLBAR`:
     - **A** (Standard): Symbol und Text;
     - **B** (`text`): nur Text.
4. **Seitenleiste:**
   - leicht warmes Weiß, Zeilen 22 px, 16-px-Symbole;
   - Abschnittsüberschriften fett in gedämpftem Text;
   - Auswahl als sanfte Akzentfläche mit 1-px-Akzentrand (nicht vollflächig orange).
   - Nicht angepasste Orte zeigen das Themensymbol (`user-home`, `user-desktop`, `folder-documents` …, `place_path_icon_names`); angepasste Orte (Farbe, eigenes Symbol) bleiben wie bisher.
   - Zusätzlich gibt es Themensymbole für „Zuletzt verwendet“ (`document-open-recent`) und Laufwerke (`drive-harddisk`).
5. **Dateiliste:**
   - quadratische, volle Zeilenbreite wie die GTK-2-Baumansicht;
   - Spaltenköpfe mit leichtem Verlauf, Trennlinien und normaler Schriftstärke;
   - sehr dezentes Überfahren;
   - Auswahl als Akzentfläche (28 %) mit Akzentrand, Text und Symbole behalten ihre Farbe.
6. **Menüs:** Die Hervorhebung von Menüleiste und Menüeinträgen entspricht den Panelmenüs (Akzentverlauf, helle Schrift); die Menüfläche ist hell mit Schatten.
7. **Pfad und Bereichstitel:** Der aktuelle Ordner wird fett in Textfarbe dargestellt statt in Akzentfarbe (wie Nautilus 2).

Alle CSS-Regeln bleiben auf `window.querencia-classic` begrenzt und nutzen die Farbmarken des Themas. Es gibt keine globalen Regeln.

## Standardvariante der Werkzeugleiste: A

Variante A (16-px-Symbole mit Text) ist der Standard:

- Die Human-Navigationssymbole sind klein und ruhig, die Funktionen bleiben aber sofort erkennbar (Pfeile, Haus, Neu laden).
- Das entspricht GNOMEs damaliger Einstellung „Text neben Symbolen“.

Variante B ist noch ruhiger, verliert aber die schnelle Erkennbarkeit; sie bleibt als Wahl erhalten.

## Unverändert

- Tastenkürzel und Aktionen der Werkzeugleiste.
- Menüleiste Datei/Bearbeiten/Ansicht/Gehe zu/Lesezeichen/Hilfe.
- GVfs (Papierkorb, Netzwerk), Dateioperationen und Statusleiste.
- Vorschau standardmäßig aus (QF-0e): `preferences.rs` hat außer dem Standardthema keine Änderung.

## Prüfung

Siehe . Die vollständige Testsuite lässt sich auf der VM nicht bauen: Das Testbinary überschreitet die Speichergrenze der Werkstatt-Prozessgruppe. Geprüft wurden deshalb fmt, `check --all-targets` (kompiliert alle Tests), clippy `-D warnings`, build und isolierte GUI-Tests.
