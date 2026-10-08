# SeamlessControl 0.22.1 · Windows file destination fix

> **Historical release notes / Notas históricas.** Instructions below belong to this version. For current installation and use, see [English](../README.md) / [Español](../README.es.md).

This update closes a Windows receive-path issue found during Omarchy marketplace review. A specially crafted file offer from an already paired computer could use a drive-relative name such as `C:payload.txt` to place an accepted new file outside the folder selected by the receiver. SeamlessControl now rejects drive-relative names, colons used for alternate data streams, and any name that is not a single path component **before** forming the destination path. Existing files are still never overwritten.

The fix applies to both manual file sends and copied-file transfers, including automatic approval. Normal names, including Unicode, remain supported. Linux file names containing `:` must be renamed before transfer so the same offer is safe on Windows. A Windows x64 regression test covers the reported path case; the test record remains in [the repository](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.1/docs/TEST-RESULTS.md).

**Update Omarchy:** stop active sessions, run `omarchy plugin update seamlesscontrol.control`, then `omarchy restart shell`. Open SeamlessControl and select **Update agent**. Update both Omarchy computers that exchange files.

**Update Windows:** exit SeamlessControl from the tray. Under **Assets**, download `seamlesscontrol-windows-x64.zip` and its adjacent `.sha256`, extract the ZIP, replace both `seamlesscontrol.exe` and `seamlesscontrold.exe`, then reopen the app. Pairings and preferences remain saved. The ZIP contains the two executables directly.

The release also includes the tagged source archive and its adjacent `.sha256`. See the [English user guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.1/docs/USER-GUIDE.md) or [Spanish user guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.1/docs/USER-GUIDE.es.md).

---

# SeamlessControl 0.22.1 · Corrección del destino de archivos en Windows

Esta actualización corrige una ruta de recepción Windows señalada durante la revisión del marketplace de Omarchy. Una oferta de archivo preparada por un equipo ya emparejado podía usar un nombre relativo a una unidad, como `C:payload.txt`, para crear un archivo nuevo aceptado fuera de la carpeta elegida por el receptor. SeamlessControl ahora rechaza esos nombres, los dos puntos usados para flujos alternativos y cualquier nombre que no sea un único componente de ruta **antes** de formar el destino. Los archivos existentes siguen sin sobrescribirse.

La corrección protege tanto los envíos manuales como los archivos copiados, incluida la aprobación automática. Los nombres normales, incluso Unicode, siguen admitidos. Si un archivo Linux contiene `:` en su nombre, hay que renombrarlo antes de enviarlo para mantener la oferta segura en Windows. Una prueba de regresión Windows x64 cubre el caso señalado; el [registro de pruebas](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.1/docs/TEST-RESULTS.es.md) permanece en el repositorio.

**Actualizar Omarchy:** termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control` y después `omarchy restart shell`. Abre SeamlessControl y pulsa **Actualizar agente**. Actualiza ambos equipos Omarchy que intercambian archivos.

**Actualizar Windows:** sal de SeamlessControl desde la bandeja. En **Assets**, descarga `seamlesscontrol-windows-x64.zip` y su `.sha256` contiguo, extrae el ZIP, reemplaza juntos `seamlesscontrol.exe` y `seamlesscontrold.exe` y vuelve a abrir la app. Se conservan emparejamientos y preferencias. El ZIP contiene directamente los dos ejecutables.

El release incluye también el archivo fuente etiquetado y su `.sha256` contiguo. Consulta la [guía en español](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.1/docs/USER-GUIDE.es.md) o la [guía en inglés](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.1/docs/USER-GUIDE.md).
