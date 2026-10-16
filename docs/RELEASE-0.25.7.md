# SeamlessControl 0.25.7 · Private file alerts

The marketplace reviewer traced a copied-file offer beyond SeamlessControl's notification helper: Omarchy saves notification text by passing it to a persistence subprocess. A private filename in the D-Bus notification body could therefore reappear in that subprocess's command line, even though it never appeared in the helper's arguments.

Omarchy now receives a **generic, localized desktop alert** for an incoming file. The alert still offers **Accept** and **Reject**. Open SeamlessControl's Files page to review the **filename, sender and size** before accepting; the same details remain visible in the panel's offer banner. The notification helper accepts only a bounded English/Spanish language selector and constructs its own fixed text. It rejects attempts to supply a filename or arbitrary notification body. Neither the system notification nor Omarchy's persistence command line receives file metadata. Windows keeps its existing approval dialog, which is not routed through Omarchy's notification service.

Existing pairings, permission choices, approval modes, encrypted file transfer, limits and protocol `seamlesscontrol/7` are unchanged. **Omarchy update:** finish an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and choose **Update agent** in Settings. **Windows update:** exit the tray app, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app.

A private D-Bus regression test checks both languages, the Accept/Reject action, rejection of an injected body, and the exact notification content that Omarchy would place in `/proc/<pid>/cmdline`. The Rust suite, Clippy and Windows x64 CI are recorded in the [test results](TEST-RESULTS.md). This targeted notification change has not had a new physical two-computer transfer test.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.7 · Avisos privados de archivos

El revisor del marketplace siguió la oferta de archivos más allá del auxiliar de SeamlessControl: Omarchy guarda el texto del aviso pasándolo como argumento a un subproceso. Así, un nombre privado en el cuerpo D-Bus podía reaparecer en la línea de comandos de ese proceso, aunque no estuviera en los argumentos del auxiliar.

Omarchy muestra ahora un **aviso genérico y localizado** al llegar un archivo. Conserva **Aceptar** y **Rechazar**. Abre Archivos en SeamlessControl para revisar **nombre, remitente y tamaño** antes de aceptar; los datos también siguen visibles en el banner de la oferta del panel. El auxiliar acepta solo un selector acotado de idioma inglés/español, genera texto fijo y rechaza cualquier nombre o cuerpo arbitrario. Ni el aviso del sistema ni los argumentos del proceso de persistencia de Omarchy reciben metadatos del archivo. Windows conserva su diálogo de aprobación, que no pasa por el servicio de notificaciones de Omarchy.

Se conservan emparejamientos, permisos, modos de aprobación, transferencia cifrada, límites y protocolo `seamlesscontrol/7`. **Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga el ZIP x64 y el archivo SHA-256 contiguo de este lanzamiento, verifícalo, sustituye juntos los dos ejecutables y abre la app.

Una prueba con D-Bus privado verifica ambos idiomas, los botones, el rechazo de un cuerpo inyectado y el texto exacto que Omarchy colocaría en `/proc/<pid>/cmdline`. La suite Rust, Clippy y la CI Windows x64 constan en los [resultados](TEST-RESULTS.es.md). Esta corrección concreta de avisos no ha tenido una nueva prueba física de transferencia entre dos equipos.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
