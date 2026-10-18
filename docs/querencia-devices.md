# Devices in Querencia Files

Querencia Files lists every GVfs volume and mount under **DEVICES** in the
sidebar: USB sticks and card readers, external and internal disks, optical
media, LUKS volumes and connected network shares. The row icon follows the
device kind (`src/ui/window/device_kind.rs`), using Querencia's own icons in
`data/icons/scalable/devices/`:

| Kind | Decided from | Icon |
| --- | --- | --- |
| Network drive | mount or activation root with a network scheme (`smb`, `sftp`, `dav`, …) or volume class `network` | `strata-querencia-drive-network` |
| Encrypted | LUKS hints (padlock emblems, `crypto_LUKS`, `CRYPT-` device-mapper UUID, password drives) | `strata-querencia-drive-encrypted` |
| Optical medium | `media-optical*` / `drive-optical*` icon names | `strata-querencia-media-optical` |
| Removable medium | `media-flash*`, `media-removable*`, `drive-removable-media*`, phones, cameras, or a removable drive | `strata-querencia-drive-removable` |
| Hard disk | `drive-harddisk*` (including USB disks) and everything else | `strata-querencia-drive-harddisk` |

Volumes that are not mounted are shown dimmed, and their accessible
description says so. Clicking one mounts it through GIO with a
`GtkMountOperation`; LUKS volumes ask for the passphrase in Files' unlock
dialog. Eject or unmount is available from the row's button and its context
menu. A device that is still in use reports "The device is still in use"
together with the system's message; GTK's own dialog lists the blocking
programs when GVfs can name them.

## Notifications

While Files runs, a new device mount (a mount with a GIO volume, not a
network share and not a portal or FUSE helper) is announced through
`org.freedesktop.Notifications`, for example "USB stick “NAME” mounted",
with an **Open** action that opens the medium in a new window. Mounts the
user just started from a Files window are not announced, because the window
already shows them.

## Automatic mounting

**Settings → General → Removable media → Mount removable media
automatically** (`automount_media`, on by default) mounts a volume when GIO
reports it as newly added and it can be mounted, is not mounted yet, is not a
network volume and GVfs marks it `should_automount` (GVfs leaves that off for
internal and system volumes). The mount runs through the active Files window
and never opens a window by itself.

Automatic mounting and the notifications only happen **while Querencia Files
is running**. Without a running Files, a connected stick stays unmounted
until it is opened from Files or another program.

A later step can move this out of Files: a small session service (or the
Querencia desktop, which already watches mounts for its desktop icons) that
owns the `VolumeMonitor`, mounts media with the same rules, sends the
notification and starts Files only for **Open**. Files would then drop its
own watcher so media are not mounted twice.

## Network view

GVfs lists a device once per discovery service, for example `nas-test` from
DNS-SD (Avahi) and `NAS-TEST` from WS-Discovery (wsdd). The `network:///`
listing is merged per device and protocol: names are compared without case
and without a trailing `.local`, the DNS-SD target is kept (it carries the
resolved address) and wsdd is used when it is the only source. Hovering the
icon of a merged entry names both sources and their targets. Entries for
different protocols of the same host stay separate. Servers, workgroups and
shares use Querencia's own icons with the classic theme names as fallback.
