# SeamlessControl 0.25.5 · Private file notifications

Incoming copied-file offers still show the filename, sender and size in one actionable Omarchy notification. Those details now travel to the desktop notification service over the user's D-Bus session through a helper that reads stdin. They no longer appear in `notify-send` arguments or `/proc/<pid>/cmdline`. If the notification helper cannot start, the fallback alert contains no file metadata or approval buttons; the full offer can be reviewed and answered in the Files page.

Omarchy and Windows interfaces also pass selected local file paths, destination folders and dropped file URIs to their agents over stdin instead of process arguments. The existing approval choices, peer permissions, encrypted transfer, size limits and file verification are unchanged. The command-line interface remains available for people who explicitly type a path.

**Omarchy update:** finish the active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, then choose **Update agent** in Settings. **Windows update:** exit the tray app, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app. Saved pairings and layout remain compatible; the network protocol is still `seamlesscontrol/7`.

A private D-Bus test sends a filename through stdin, checks that it reaches the notification service, selects Reject, and verifies that the filename never appears in the helper's `/proc` command line. The Rust suite and Windows build are tracked in the [test record](TEST-RESULTS.md). A new physical two-computer file-transfer test has not been performed for this targeted change.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.5 · Avisos de archivos privados

Las ofertas de archivos copiados siguen mostrando nombre, remitente y tamaño en un único aviso de Omarchy con botones de aprobación. Esos datos pasan ahora al servicio de notificaciones por la sesión D-Bus del usuario mediante un auxiliar que lee stdin. Ya no aparecen en los argumentos de `notify-send` ni en `/proc/<pid>/cmdline`. Si el auxiliar falla, el aviso alternativo no incluye metadatos ni botones de aprobación; la oferta completa se puede revisar y responder en la página Archivos.

Las interfaces de Omarchy y Windows también pasan por stdin las rutas de archivos elegidos, las carpetas de destino y las URI de archivos arrastrados, en vez de incluirlos en los argumentos del proceso. Se conservan los modos de aprobación, permisos por equipo, cifrado, límites de tamaño y verificación de archivos. La interfaz de comandos sigue disponible para quien introduzca una ruta expresamente.

**Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga de este lanzamiento el ZIP x64 y el archivo SHA-256 contiguo, verifícalo, sustituye juntos los dos ejecutables y abre la app. Se conservan emparejamientos y mapa; el protocolo sigue siendo `seamlesscontrol/7`.

Una prueba con D-Bus privado envía un nombre por stdin, comprueba que llega al servicio de notificaciones, elige Rechazar y verifica que el nombre nunca figura en `/proc` para el auxiliar. La suite Rust y la compilación Windows constan en el [registro de pruebas](TEST-RESULTS.es.md). No se ha hecho una nueva prueba física de transferencia entre dos equipos para esta corrección concreta.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
