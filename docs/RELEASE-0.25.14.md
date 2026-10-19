# SeamlessControl 0.25.14 · Omarchy clipboard recovery

Copying a selection from Omarchy could stop synchronizing new text and images when its Wayland clipboard provider stalled. This update bounds every external clipboard read by time and size, even if another process keeps the output pipe open. It also limits the clipboard watcher callback. An unresponsive selection is skipped; subsequent copies can synchronize without restarting the control session.

- Covers text, image and file selections on the Omarchy sending side.
- Existing pairings, peer permissions, file approval and the control protocol remain compatible.
- Flea does not need to be removed; the recovery applies to clipboard providers generally.

Regression tests cover a stalled clipboard provider and a child that inherits its output pipe. Linux tests and Windows x64 CI passed on `alpha`; the [test record](TEST-RESULTS.md) distinguishes those checks from physical two-computer testing. A physical Omarchy-to-Omarchy text and image paste succeeded after freeing a stalled observer before the final deadline regression was added. This exact release build has not received a new physical two-computer test.

**Update Omarchy:** end the active session, run `omarchy plugin update seamlesscontrol.control --yes`, then `omarchy restart shell`. In SeamlessControl **Settings**, select **Update agent**, wait for the terminal to finish, and restart **Receive control** or **Connect**. Update both Omarchy computers for symmetric clipboard recovery.

**Windows x64:** its clipboard code is unchanged in this release. The release still provides a version-matched ZIP containing `seamlesscontrol.exe` and `seamlesscontrold.exe`. Download the ZIP and its adjacent `.sha256` file from **Assets**, verify the hash, exit the app from the tray, replace both executables together, and reopen it.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.14 · Recuperación del portapapeles de Omarchy

Al copiar una selección desde Omarchy, la sincronización de texto e imágenes nuevos podía detenerse si el proveedor Wayland del portapapeles se quedaba bloqueado. Esta versión limita por tiempo y tamaño cada lectura externa, incluso cuando otro proceso mantiene abierto el canal de salida. También limita el tiempo de la llamada del observador. Se omite la selección que no responde y las siguientes copias pueden sincronizarse sin reiniciar la sesión de control.

- Cubre selecciones de texto, imágenes y archivos enviadas desde Omarchy.
- Conserva la compatibilidad de los emparejamientos, permisos, aprobación de archivos y protocolo de control.
- No hace falta quitar Flea: la recuperación se aplica a los proveedores del portapapeles en general.

Las pruebas de regresión cubren un proveedor bloqueado y un proceso hijo que conserva abierto su canal de salida. Pasaron las pruebas Linux y la CI Windows x64 en `alpha`; el [registro](TEST-RESULTS.es.md) distingue estas verificaciones de las pruebas físicas. Antes de añadir la última prueba del plazo, se comprobó físicamente el pegado de texto e imágenes entre dos Omarchy tras liberar un observador bloqueado. Esta compilación exacta no ha tenido una nueva prueba física entre dos equipos.

**Actualiza Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y después `omarchy restart shell`. En **Ajustes** de SeamlessControl pulsa **Actualizar agente**, espera a que acabe la terminal y reinicia **Recibir control** o **Conectar**. Actualiza ambos equipos Omarchy para recuperar la sincronización en los dos sentidos.

**Windows x64:** su código de portapapeles no cambia en esta versión. El lanzamiento incluye igualmente un ZIP con `seamlesscontrol.exe` y `seamlesscontrold.exe` de la misma versión. Descarga el ZIP y su archivo `.sha256` contiguo desde **Assets**, verifica la suma, sal de la aplicación desde la bandeja, sustituye juntos ambos ejecutables y vuelve a abrirla.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
