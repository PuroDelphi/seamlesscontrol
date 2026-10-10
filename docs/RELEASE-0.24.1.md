# SeamlessControl 0.24.1 · Safer clipboard handling in the Omarchy panel

This maintenance release addresses the [marketplace review finding](https://github.com/omacom/omarchy-plugin-marketplace/issues/10961#issuecomment-6102314469) concerning Qt 6.12 and foreign Wayland text selections. The Omarchy panel's address, port, size and file-path fields are now natively read-only. They still accept keyboard edits, while **Ctrl+V** reads text only when requested through a separate process limited to 4 KiB, two seconds and 256 MiB of address space. Oversized, stalled, multiline and invalid UTF-8 selections are ignored. The file and folder picker remains available.

The pairing and encrypted network protocols are unchanged. Existing paired computers do not need to pair again. Windows controls and the 0.24.0 features remain unchanged. Local checks covered keyboard entry and selection replacement in Qt, isolated QML rendering, and bounded-paste regression cases. The physical cross-computer results for 0.24.0 are recorded in the [test results](TEST-RESULTS.md); this patch does not claim a new physical test.

**Update Omarchy:** end active sessions, run `omarchy plugin update seamlesscontrol.control --yes` and `omarchy restart shell`, then select **Update agent** in the panel. Check that plugin and agent both show 0.24.1. **Update Windows x64:** download the release ZIP and adjacent `.sha256` file, then use **Settings → Update from release ZIP…**. Windows was rebuilt for version alignment; this patch does not change its control workflow.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [0.24.0 feature release](RELEASE-0.24.0.md)

---

# SeamlessControl 0.24.1 · Lectura segura del portapapeles en el panel Omarchy

Esta versión correctiva atiende el [hallazgo de la revisión del marketplace](https://github.com/omacom/omarchy-plugin-marketplace/issues/10961#issuecomment-6102314469) sobre Qt 6.12 y selecciones de texto Wayland de otras aplicaciones. Los campos de dirección, puerto, tamaño y rutas del panel Omarchy están ahora internamente en solo lectura. Siguen admitiendo escritura con el teclado; **Ctrl+V** lee texto solo al solicitarlo, mediante un proceso separado limitado a 4 KiB, dos segundos y 256 MiB de espacio de direcciones. Se ignoran las selecciones demasiado grandes, bloqueadas, multilínea o con UTF-8 inválido. El selector de archivos y carpetas sigue disponible.

El emparejamiento y los protocolos cifrados de red no cambian. Los equipos ya emparejados no tienen que volver a emparejarse. Los controles de Windows y las características de 0.24.0 se mantienen. Las comprobaciones locales cubrieron escritura con teclado y sustitución de selección en Qt, renderizado QML aislado y los casos de regresión del pegado limitado. Las pruebas físicas de 0.24.0 figuran en el [registro de pruebas](TEST-RESULTS.es.md); esta corrección no afirma una nueva prueba física.

**Actualizar Omarchy:** termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y `omarchy restart shell`, luego pulsa **Actualizar agente** en el panel. Comprueba que plugin y agente muestren 0.24.1. **Actualizar Windows x64:** descarga el ZIP del lanzamiento y su `.sha256` contiguo; después usa **Ajustes → Actualizar desde el ZIP de lanzamiento…**. Windows se recompiló para mantener la misma versión; esta corrección no cambia su flujo de control.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Características de 0.24.0](RELEASE-0.24.0.md)
