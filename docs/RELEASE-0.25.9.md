# SeamlessControl 0.25.9 · SVG clipboard memory protection

The marketplace reviewer found that a small SVG clipboard payload could request many filter primitives. Each primitive may allocate a full-size RGBA surface inside the renderer, so the existing outer-canvas and input-size limits did not bound memory use.

The agent now **rejects SVGs containing filters before rendering**. Ordinary vector SVGs and the supported raster clipboard formats continue to work. If an application also offers a raster copy of a filtered SVG, SeamlessControl prefers that bounded raster representation. This rule applies to the shared image conversion path on Omarchy and Windows. It does not change pairing, permissions or protocol `seamlesscontrol/7`.

**Omarchy update:** end an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and choose **Update agent** in Settings. **Windows update:** exit the tray app, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app. Update both agents to receive the SVG protection at both clipboard sources.

Regression tests verify that a large-canvas SVG with referenced filter primitives is rejected before rendering, while an ordinary vector SVG still converts to PNG. Linux tests, Clippy and Windows x64 CI are recorded in the [test results](TEST-RESULTS.md). This targeted security fix has not had a new physical two-computer test.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.9 · Protección de memoria del portapapeles SVG

El revisor del marketplace encontró que un SVG pequeño copiado al portapapeles podía solicitar muchas primitivas de filtro. Cada primitiva puede reservar una superficie RGBA completa dentro del renderizador, por lo que los límites del lienzo final y del tamaño de entrada no acotaban el uso de memoria.

El agente ahora **rechaza los SVG que contienen filtros antes de renderizarlos**. Los SVG vectoriales normales y los formatos ráster compatibles siguen funcionando. Si una aplicación ofrece además una copia ráster del SVG filtrado, SeamlessControl prefiere esa representación acotada. La regla se aplica a la ruta compartida de conversión de imágenes en Omarchy y Windows. No cambia el emparejamiento, los permisos ni el protocolo `seamlesscontrol/7`.

**Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga el ZIP x64 y el SHA-256 contiguo de este lanzamiento, verifícalo, sustituye juntos los dos ejecutables y abre la app. Actualiza ambos agentes para aplicar la protección SVG a ambos orígenes del portapapeles.

Las pruebas verifican que un SVG de lienzo grande con primitivas de filtro referenciadas se rechaza antes de renderizar, mientras que uno vectorial normal aún se convierte en PNG. Las pruebas Linux, Clippy y CI Windows x64 constan en los [resultados](TEST-RESULTS.es.md). Esta corrección de seguridad concreta no ha tenido una nueva prueba física entre dos equipos.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
