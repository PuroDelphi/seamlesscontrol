# SeamlessControl 0.25.12 · Isolated image decoding

The marketplace reviewer found that usvg copied an inherited `stroke-dasharray` for every child path **before** the previous render-tree budget. We also reviewed the native raster path: the locked `image` crate documents its `max_alloc` as non-strict for some decoders. This release moves non-PNG clipboard conversion out of the input-control agent on Omarchy and Windows.

- SVG preflight rejects inherited dash arrays, inline CSS and `<style>` before usvg can expand them. Simple vector SVG remains available, along with the supported raster clipboard formats.
- Non-PNG image data is decoded in a separate agent process. Linux applies an address-space limit; Windows places the child in a memory-limited Job Object. Both have a 768 MiB process limit and a ten-second wall deadline. Input arrives over bounded stdin and output is bounded to the 32 MiB PNG wire limit. A costly or stalled conversion loses that clipboard update while the mouse, keyboard and connection remain in the main agent.
- The existing SVG source, XML, geometry, tree and render budgets remain defense in depth. PNG already on the wire is passed through after header and size checks without invoking a native source decoder.

The tests run the real helper executable: a small JPEG and SVG convert, inherited dash arrays are rejected immediately, and a deliberately stalled decoder is killed while the parent exits within the deadline. Core, Linux and Windows x64 CI results are linked in the [test record](TEST-RESULTS.md). No new physical two-computer test was performed for this security change. Pairings and protocol `seamlesscontrol/7` are unchanged.

**Omarchy:** end an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and select **Update agent** in Settings. **Windows:** exit from the tray, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app. Update both ends to protect both clipboard sources.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.12 · Decodificación de imágenes aislada

El revisor del marketplace encontró que usvg copiaba un `stroke-dasharray` heredado por cada trazado hijo **antes** del límite del árbol de renderizado. Revisamos también la ruta ráster nativa: la biblioteca `image` fijada documenta que `max_alloc` no es estricto para ciertos decodificadores. Este lanzamiento saca la conversión de imágenes no PNG del agente que controla la entrada en Omarchy y Windows.

- El análisis previo de SVG rechaza trazos discontinuos heredados, CSS en atributos y elementos `<style>` antes de que usvg los expanda. Siguen disponibles los SVG vectoriales sencillos y los formatos ráster admitidos.
- Los datos de imagen no PNG se decodifican en otro proceso del agente. Linux limita su espacio de direcciones; Windows coloca el proceso en un Job Object con límite de memoria. Ambos tienen 768 MiB de límite y diez segundos de plazo. La entrada llega por stdin acotado y la salida se limita a 32 MiB de PNG. Si una conversión consume demasiado o se atasca, se descarta esa actualización del portapapeles mientras el ratón, el teclado y la conexión siguen en el agente principal.
- Permanecen como defensa adicional los límites de SVG para origen, XML, geometría, árbol y renderizado. Los PNG que ya están en el formato de intercambio pasan tras verificar encabezado y tamaño, sin decodificar el formato de origen en el agente.

Las pruebas ejecutan el auxiliar real: un JPEG y un SVG pequeños se convierten, el trazo discontinuo heredado se rechaza de inmediato y un decodificador bloqueado se termina dentro del plazo. Los resultados de CI del núcleo, Linux y Windows x64 están en el [registro](TEST-RESULTS.es.md). No se hizo una nueva prueba física entre dos equipos para esta corrección. Se mantienen los emparejamientos y el protocolo `seamlesscontrol/7`.

**Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Windows:** sal desde la bandeja, descarga el ZIP x64 del lanzamiento y su SHA-256 contiguo, verifícalo, sustituye juntos ambos ejecutables y abre la app. Actualiza ambos extremos para proteger los dos orígenes del portapapeles.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
