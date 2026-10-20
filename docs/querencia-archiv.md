# Querencia Archiv in Files

When Querencia Archiv is installed, its program `querencia-archiv` in `PATH`
or its launcher `org.querencia.Archiv.desktop`, **Extract here** hands the
archive to it: `querencia-archiv --entpacken-hier <archive>`. Archiv shows its
own progress, conflict, password and error windows. This applies to local
files whose type, guessed from the name, is one of the launcher's MimeType
entries (exactly, so OpenDocument files are not archives); without the
launcher the 14 types of Archiv 0.1.0 apply. In the context menu **Extract
here** therefore also appears for types the built-in support cannot read,
such as `.tar.xz` or `.tar.zst`.

**Compress…** and **Extract to…** keep using Files' built-in archive support
(`archives.md`), which writes 7z and password-protected archives that Archiv
0.1.0 cannot create. The keyboard command for extracting accepts the built-in
formats and then hands them to Archiv as well. Archiv is looked up each time,
so installing or removing it needs no restart of Files.
