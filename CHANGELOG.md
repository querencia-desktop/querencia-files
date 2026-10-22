# Changelog

Querencia Files stages (`files/qf-…` tags). Earlier stages (QF-0a to QF-0s)
are summarized in [docs/querencia-lizenz.md](docs/querencia-lizenz.md).

## QF-0t-3 – 2026-10-22

- `.clean-check-ignore` lists the two upstream files with known test data
  (signing test key, emoji ZWJ sequence). No code change.

## QF-0t-2 – 2026-10-22

- QF-0t merged with the `querencia` branch (integration folders QF-0r-1 and
  QF-0s), so the build descends from the live line. No code change.

## QF-0t – 2026-10-22

- About page and Help menu read the Querencia Desktop release from
  `/usr/share/querencia/release` (`VERSION`, `HOME_URL`, `BUG_REPORT_URL`,
  `DOCUMENTATION_URL`), then `/usr/share/querencia/version`, then built-in
  defaults. Packages no longer need to patch the release name.
- The default theme "Querencia Klassisch" follows the GTK theme: under a dark
  GTK theme (colour scheme "dark" with Querencia-Klassisch 0.2 or later) Files
  takes its colours from the theme's named colours and switches while running;
  under a light theme the palette is unchanged.
- Application ID `org.querencia.Files` (was `io.github.lgse.Strata`): desktop
  entry `org.querencia.Files.desktop`, window class, D-Bus name, file chooser
  `org.querencia.Files.FileChooser`, portal backend `querencia`, resource path
  `/org/querencia/Files`.
- Per-user directories are `querencia-files` under the XDG config, state, data
  and cache directories. An existing `strata` directory is moved on first
  start and left as a link, so older builds still read the same files.
- Package marker `share/querencia-files/install-source.toml` (the old
  `share/strata/` location is still read).
- Remaining Strata names in paths, log messages and update links replaced;
  Strata stays named as the origin in the About page and the license.
