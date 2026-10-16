# SeamlessControl 0.25.10 · Comprehensive SVG clipboard limits

The marketplace reviewer confirmed the filter fix but found that deeply nested opacity groups could still keep many full-canvas RGBA layers alive inside resvg. We inspected the other allocation paths in the locked resvg 0.48.1 renderer and replaced the single filter check with **a shared pre-render resource policy** for SVG clipboard data on Omarchy and Windows.

- Before usvg parses an SVG, the agent limits the XML source to 2 MiB, 4,096 elements and depth 64.
- Before `resvg::render`, it accounts for the output surface and each nested isolated group's conservative transformed layer size. The live RGBA surface budget is 128 MiB; expanded render-tree nodes and depth are limited too.
- Filters, masks, clipping paths, patterns and embedded image nodes are rejected. These renderer features can allocate additional recursive pixmaps that are not covered by the group budget. Ordinary vector shapes, gradients, text and modest opacity groups remain available; bounded raster clipboard formats remain available as an alternative.

This is deliberately conservative: a complex SVG that exceeds these limits is ignored as clipboard pixels. To preserve its appearance or structure, copy a raster version or transfer the original SVG file through the approved Files flow. Pairing, peer permissions and protocol `seamlesscontrol/7` are unchanged.

**Omarchy update:** end an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and choose **Update agent** in Settings. **Windows update:** exit the tray app, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app. Both clipboard sources need the updated agent.

Regression tests cover nested full-canvas opacity layers, modest layers, filters, masks, clipping paths, patterns, deep XML and excessive element counts. Linux and Windows x64 CI results are recorded in the [test results](TEST-RESULTS.md). No new physical two-computer test was performed for this targeted security fix.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.10 · Límites integrales para SVG

El revisor del marketplace confirmó el arreglo de filtros, pero encontró que los grupos de opacidad muy anidados aún podían mantener vivas muchas capas RGBA del tamaño del lienzo dentro de resvg. Revisamos las otras rutas de reserva de memoria del renderizador resvg 0.48.1 fijado y sustituimos la comprobación individual de filtros por **una política de recursos previa al renderizado** compartida por Omarchy y Windows.

- Antes de que usvg analice el SVG, el agente limita el XML a 2 MiB, 4.096 elementos y profundidad 64.
- Antes de `resvg::render`, contabiliza la superficie de salida y el tamaño conservador de la capa transformada de cada grupo aislado anidado. El presupuesto de superficies RGBA vivas es 128 MiB; también se limitan los nodos y la profundidad del árbol expandido.
- Se rechazan filtros, máscaras, recortes, patrones y nodos de imagen incrustada. Estas funciones del renderizador pueden reservar otras superficies recursivas que no cubre el presupuesto de grupos. Las formas vectoriales normales, degradados, texto y grupos de opacidad modestos siguen disponibles; también permanecen los formatos ráster acotados como alternativa.

El límite es conservador deliberadamente: un SVG complejo que lo supere se omite como píxeles del portapapeles. Para conservar su aspecto o estructura, copia una versión ráster o transfiere el SVG original mediante el flujo de Archivos con aprobación. Emparejamiento, permisos y protocolo `seamlesscontrol/7` no cambian.

**Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga el ZIP x64 y el SHA-256 contiguo de este lanzamiento, verifícalo, sustituye juntos los dos ejecutables y abre la app. Ambos orígenes del portapapeles necesitan el agente actualizado.

Las pruebas cubren grupos de opacidad anidados de lienzo completo, grupos modestos, filtros, máscaras, recortes, patrones, XML profundo y exceso de elementos. Los resultados de CI Linux y Windows x64 constan en los [resultados](TEST-RESULTS.es.md). Esta corrección de seguridad concreta no tuvo una nueva prueba física entre dos equipos.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
