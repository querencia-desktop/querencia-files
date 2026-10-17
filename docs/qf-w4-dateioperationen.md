# Querencia Files W4 – klassische Dateioperationen

Stand 2026-10-17, Basis QF-0f. Ziel: Bedienung wie Nautilus 2.x (GNOME 2 /
Ubuntu 10.04). Der Strata-Kern bleibt erhalten; geändert wurde nur, was fehlte
oder vom GNOME-2-Verhalten abwich.

## Bestand vor W4 (geprüft im isolierten Profil mit dem QF-0f-Binary)

| Funktion | Kategorie | Befund |
|---|---|---|
| Strg+C / Strg+X / Strg+V / Strg+A | A | vorhanden; Verschieben erst beim Einfügen |
| Zwischenablage | C | nur GDK-Dateiliste (`text/uri-list`); Ausschneiden für andere Dateimanager unsichtbar, fremdes „cut“ wurde kopiert |
| F2, Umbenennen im Kontextmenü | A | Inline-Umbenennen; Konflikt, leerer/ungültiger Name, fehlende Rechte abgefangen |
| Strg+R | C | benannte um (Strata), Nautilus 2 lädt neu |
| Entf / Umschalt+Entf | A | Papierkorb über GIO; dauerhaftes Löschen mit Abfrage |
| Papierkorb wiederherstellen | A | Kontextmenü „Wiederherstellen“ mit Bestätigung |
| Alt+Enter / Eigenschaften | A | Dialog mit Typ, Ort, Größe (Ordner asynchron), Datum, Rechten |
| Alt+←/→/↑, Alt+Pos1, Strg+L, F5, Strg+Umschalt+N | A | vorhanden |
| Strg+N | D | im Standard-Tastenmodell nicht belegt |
| Menü „Datei“/„Bearbeiten“ | B | ohne „Neues Fenster“, ohne „Endgültig löschen“, ohne Tastenkürzel, nichts ausgegraut |
| „Symbole anordnen“ | B | Sortierung nur über das Sortiersymbol im Bereichskopf |
| Kontextmenü-Reihenfolge | C | „Neuer Ordner mit Auswahl“ ganz oben, „Eigenschaften“ mitten im Menü |
| Öffnen mit Standardanwendung | A | GIO-MIME-Zuordnung |
| GVfs `trash:///`, `network:///` | A | vorhanden |

## Änderungen

- **Zwischenablage:** zusätzlich `x-special/gnome-copied-files`
  (`copy`/`cut` + URIs, wie Nautilus, Caja, Nemo, Thunar, PCManFM) und beim
  Ausschneiden `application/x-kde-cutselection`. Beim Einfügen aus einer fremden
  Anwendung wertet Querencia Files die Cut-Marke aus; verschoben wird erst beim
  Einfügen, über den vorhandenen Strata-Transfer. Hinweis: Der in Auftrag und
  Literatur genannte Name „application/x-special-gnome-copied-files“ ist kein
  verbreitetes Format; maßgeblich ist `x-special/gnome-copied-files`.
- **Tasten:** Strg+R lädt neu (F2 bleibt Umbenennen), Strg+N öffnet ein neues
  Fenster am aktuellen Ort. Im 10xer-Modus bleiben beide wie bisher.
- **Klassisches Menü:** Datei ▸ Neues Fenster; Bearbeiten ▸ Umbenennen …,
  Endgültig löschen; Ansicht ▸ Symbole anordnen (Name, Größe, Typ, Geändert,
  Umkehren). Jeder Eintrag zeigt sein Tastenkürzel in Strata-Schreibweise
  („Strg+X“, „Entf“); die Anzeige bindet keine Taste zusätzlich. Ausschneiden,
  Kopieren, Duplizieren, Umbenennen und Löschen sind ohne Auswahl ausgegraut,
  Einfügen ohne Dateien in der Zwischenablage.
- **Kontextmenüs:** GNOME-2-Reihenfolge (Öffnen zuerst, Eigenschaften zuletzt),
  „Umbenennen …“; Freifläche: Neuer Ordner, Neue Datei | Öffnen mit, Terminal |
  Einfügen | Alles auswählen, Symbole anordnen ▸, Aktualisieren, …, Eigenschaften.
- **Darstellung:** Eigenschaften zeigen das Dateitypsymbol des Symbolthemas
  (GNOME Human). Ausgeschnittene Dateien werden im klassischen Fenster blass
  gezeichnet, kopierte bleiben unverändert (statt Scheren-/Kopie-Glyphe).
