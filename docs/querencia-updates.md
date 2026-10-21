# Updates in Querencia Files

Querencia Files is updated through the system's package source (APT), together
with the rest of Querencia Desktop. Strata's own updater is switched off by
the constant `SELF_UPDATE` in `src/services/mod.rs`:

- no check for new releases, neither at startup nor later (Strata checked
  GitHub releases, the AUR or pacman repositories, at most every 6 hours);
- no "Update available" entry at the bottom of the sidebar;
- no release notes download and no in-place installer: the entry points
  `check_for_updates`, `fetch_release_notes` and `install_update` return
  without starting a thread or opening a connection;
- the Updates page in Settings only says that updates come from the package
  source; it has no switch and no release channel. A saved
  `check_for_updates` preference is kept but has no effect.

The upstream code stays in the tree, unchanged except for these guards, so
later Strata versions merge without conflicts in the updater itself.
