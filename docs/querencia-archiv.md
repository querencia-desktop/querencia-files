# Querencia Archiv in Files

When Querencia Archiv is installed, its program `querencia-archiv` in `PATH`
or its launcher `org.querencia.Archiv.desktop`, Files hands two archive
commands to it; Archiv shows its own progress, conflict, password and error
windows. Without Archiv, Files' built-in archive support (`archives.md`)
stays as it is.

| Command | With Archiv |
| --- | --- |
| **Extract here** | `querencia-archiv --entpacken-hier <archive>` for local files whose type, guessed from the name, is one of the launcher's MimeType entries (exactly, so OpenDocument files are not archives). Without the launcher the 14 types of Archiv 0.1.0 apply. |
| **Compress…** | Files asks for the name and the format Archiv writes (ZIP, TAR.GZ, TAR.XZ, no password), refuses an existing name, then runs `querencia-archiv --erstellen <folder>/<name>.<format> <items>`. The archive is created next to the first item. |

**Extract to…** and the keyboard commands for archives keep using the built-in
support; **Extract here** from the keyboard also goes to Archiv. Archiv is
looked up each time, so installing or removing it needs no restart of Files.
