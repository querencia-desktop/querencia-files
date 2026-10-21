# Licensing of the Querencia additions

Querencia Files is a fork of [Strata](https://github.com/lgse/strata), which
is licensed under the [MIT License](../LICENSE) (Copyright (c) 2026 LGSE
Ltd.). The Querencia changes are released under the same MIT License, so the
whole program keeps a single license.

## Files added by Querencia

Copyright 2026 Querencia Desktop contributors, MIT License. The Rust files
carry `SPDX-License-Identifier: MIT` and
`SPDX-FileCopyrightText: 2026 Querencia Desktop contributors`. The style
sheet, the icons and the documents below are embedded in the program or read
as data; they keep their content unchanged and are covered by this notice:

- `data/icons/scalable/devices/strata-querencia-drive-encrypted.svg`
- `data/icons/scalable/devices/strata-querencia-drive-harddisk.svg`
- `data/icons/scalable/devices/strata-querencia-drive-network.svg`
- `data/icons/scalable/devices/strata-querencia-drive-removable.svg`
- `data/icons/scalable/devices/strata-querencia-media-optical.svg`
- `data/icons/scalable/devices/strata-querencia-network-server.svg`
- `data/icons/scalable/devices/strata-querencia-network-share.svg`
- `data/icons/scalable/devices/strata-querencia-network-workgroup.svg`
- `docs/qf-0f-visuelle-ueberarbeitung.md`
- `docs/qf-w4-dateioperationen.md`
- `docs/querencia-devices.md`
- `src/adapters/local_files/network_devices.rs`
- `src/adapters/local_files/network_devices/tests.rs`
- `src/classic.css`
- `src/sandbox_helper/raster.rs`
- `src/sandbox_helper/raster/tests.rs`
- `src/ui/browser/network.rs`
- `src/ui/browser/network/tests.rs`
- `src/ui/theme_icons.rs`
- `src/ui/theme_icons/tests.rs`
- `src/ui/window/composition/classic.rs`
- `src/ui/window/device_kind.rs`
- `src/ui/window/device_kind/tests.rs`
- `src/ui/window/device_watch.rs`
- `src/ui/window/device_watch/tests.rs`
- `docs/querencia-archiv.md`
- `src/ui/browser/querencia_archive.rs`
- `src/ui/browser/querencia_archive/tests.rs`
- `docs/querencia-updates.md`
- `src/ui/settings/package_updates.rs`
- `src/ui/settings/about/tests.rs`
- `docs/querencia-lizenz.md` (this file)

Upstream Strata files that Querencia changed (QF-0a … QF-0r) keep their
upstream notice; the Querencia changes in them are MIT-licensed as well. The
history of every change is in the Git log (commits starting with `QF-` or
`feat(querencia)`).

## Name and origin in the program

Since QF-0r everything the program shows calls it "Querencia Files". Strata is
named only as its origin: the About page (Settings → About) shows "Based on
Strata by LGSE Ltd. (MIT)", the Strata version it is based on, and expands to
the full text of `LICENSE` (embedded in the program), and "Copy version info"
ends with "Based on Strata <version> (LGSE Ltd., MIT)". The application ID
`io.github.lgse.Strata`, resource paths, D-Bus names, file names and internal
identifiers still carry the Strata name; renaming them is a separate step.
