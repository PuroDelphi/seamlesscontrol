# SeamlessControl 0.24.0 · One workspace across Omarchy and Windows

Move a physical mouse and keyboard between paired Omarchy and Windows computers by crossing the screen edge. Share text during the connection, and copy files or folders in one file manager and paste them in the other. The receiving computer remains in charge of file approval.

## What's new

- **A whole folder in one offer.** Copy a folder, nested files, or several selected files. The receiver sees one offer with the item count and total size, approves once, and pastes the verified group. The existing single-file flow still works. Groups are limited to 256 entries and the configured file-size limit; symbolic links are rejected.
- **Sharing follows the connection.** Pairing establishes trust but does not start file exchange. Copied and manually sent files require a live, authenticated control session at both ends. Ending the session stops new offers and transfers. Control, text and files can also be allowed or denied separately for each paired computer; both apps show the last connection time.
- **Choose the crossing gesture.** Fluid crossing remains the default. Deliberate mode requires two edge crossings within 1.6 seconds; Protect full-screen requires that gesture only while a full-screen window is active. Edge return and physical Escape remain available.
- **Recover local input.** The source releases capture when its desktop locks or sleeps. After unlocking or waking, the physical mouse and keyboard stay local until a new edge crossing. Stopping the receiver returns control and a later reconnection also requires a fresh crossing.
- **Clearer diagnosis and updates.** Both interfaces can check a receiver's control port and saved pairing before retrying. Omarchy shows plugin and agent versions and confirms an agent update after completion. Windows shows app and agent versions and can update both executables together from a tagged release ZIP after checking its adjacent SHA-256 file.
- **End the Omarchy session from either monitor.** Every bar panel sees an active source session and can stop the same agent. Ending it releases capture and ends file sharing.

## Verified on physical computers

On an Omarchy computer and a Windows 11 x64 computer, mouse, click, keyboard, text clipboard and return by edge worked in both directions. Both crossing modes were exercised from each source. Per-computer Control, Text and Files permissions were checked on each receiver. Folders, nested files and multiple selections were copied and pasted in both directions with one approval; rejection, size limits and symbolic-link rejection were checked. Disconnected computers did not exchange files despite remaining paired. The physical source also retained local input after lock, unlock, sleep, wake and receiver interruption in both directions. The [test record](TEST-RESULTS.md) gives the observed scope and historical detail.

## Install or update

**Omarchy:** install with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, open the panel and choose **Install agent**. To update, end sessions, run `omarchy plugin update seamlesscontrol.control --yes` and `omarchy restart shell`, then choose **Update agent** in the panel. Check that both displayed versions are 0.24.0.

**Windows x64:** download `seamlesscontrol-windows-x64.zip` and `seamlesscontrol-windows-x64.zip.sha256` from this release's **Assets**. For a new installation, extract the ZIP and open `seamlesscontrol.exe`. For an existing installation, use **Settings → Update from release ZIP…** and select the downloaded ZIP; the app verifies its checksum and replaces the GUI and agent together. Keep both executables in the same folder. Pairings and preferences remain saved.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Spanish README](../README.es.md)

---

# SeamlessControl 0.24.0 · Un espacio de trabajo entre Omarchy y Windows

Mueve el ratón y teclado físicos entre equipos Omarchy y Windows emparejados al cruzar el borde de la pantalla. Comparte texto durante la conexión y copia archivos o carpetas en un explorador para pegarlos en el otro. El receptor conserva el control de la aprobación de archivos.

## Novedades

- **Una carpeta en una sola oferta.** Copia una carpeta, archivos anidados o varios archivos seleccionados. El receptor ve la cantidad de elementos y el tamaño total, aprueba una sola vez y pega el grupo verificado. La copia de un archivo sigue funcionando. El grupo admite hasta 256 entradas y respeta el límite de tamaño configurado; se rechazan enlaces simbólicos.
- **El intercambio depende de la conexión.** Emparejar establece confianza, pero no comparte archivos. Copiar o enviar archivos manualmente exige una sesión de control autenticada y activa en ambos extremos. Terminarla detiene las ofertas y transferencias nuevas. Se pueden permitir o denegar por separado Control, Texto y Archivos para cada equipo emparejado; las dos interfaces muestran la fecha de la última conexión.
- **Elige el gesto de cruce.** El modo fluido sigue siendo el predeterminado. El modo deliberado requiere dos cruces del borde en 1,6 segundos; Proteger pantalla completa lo exige solo si hay una ventana a pantalla completa. Siguen disponibles el regreso por el borde y Escape físico.
- **Recupera la entrada local.** El origen libera la captura cuando su escritorio se bloquea o suspende. Tras desbloquear o despertar, el ratón y teclado físicos permanecen locales hasta un cruce nuevo. Detener el receptor devuelve el control y una reconexión posterior también exige cruzar otra vez.
- **Diagnóstico y actualización más claros.** Ambas interfaces comprueban el puerto de control y el emparejamiento guardado antes de reintentar. Omarchy muestra las versiones del plugin y agente y confirma la actualización al terminar. Windows muestra las versiones de app y agente y puede actualizar ambos ejecutables juntos desde un ZIP de lanzamiento con su archivo SHA-256 contiguo.
- **Termina la sesión Omarchy desde cualquiera de sus monitores.** Los paneles de ambas barras ven la misma sesión de origen y pueden detener el agente. Al hacerlo, se libera la captura y termina el intercambio de archivos.

## Verificado en equipos físicos

Entre un Omarchy y un Windows 11 x64 funcionaron el ratón, clic, teclado, portapapeles de texto y regreso por el borde en ambos sentidos. Se probaron ambos modos de cruce desde cada origen. Los permisos Control, Texto y Archivos se verificaron en cada receptor. Carpetas, archivos anidados y selecciones múltiples se copiaron y pegaron en ambos sentidos con una aprobación; también se comprobaron rechazo, límite de tamaño y rechazo de enlaces simbólicos. Los equipos desconectados no compartieron archivos pese a seguir emparejados. El origen conservó la entrada local tras bloqueo, desbloqueo, suspensión, reanudación e interrupción del receptor en ambos sentidos. El [registro de pruebas](TEST-RESULTS.es.md) detalla el alcance observado y el historial.

## Instalar o actualizar

**Omarchy:** instala con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, abre el panel y pulsa **Instalar agente**. Para actualizar, termina las sesiones, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y `omarchy restart shell`, luego pulsa **Actualizar agente**. Comprueba que las dos versiones mostradas sean 0.24.0.

**Windows x64:** descarga `seamlesscontrol-windows-x64.zip` y `seamlesscontrol-windows-x64.zip.sha256` de **Assets** en este lanzamiento. Para instalar por primera vez, extrae el ZIP y abre `seamlesscontrol.exe`. Para actualizar, usa **Ajustes → Actualizar desde el ZIP de lanzamiento…** y elige el ZIP descargado; la app verifica su checksum y reemplaza juntos la interfaz y el agente. Conserva ambos ejecutables en la misma carpeta. Se mantienen los emparejamientos y preferencias.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [README en inglés](../README.md)
