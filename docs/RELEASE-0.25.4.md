# SeamlessControl 0.25.4 · Bounded SVG clipboard rendering

Clipboard SVGs now render only their vector content. Embedded images referenced through `data:` URLs or paths are omitted, and the SVG renderer is built without its raster-image decoder. This closes a path where an embedded animated WebP could allocate far more memory than the SVG canvas limit allowed. Ordinary WebP, PNG, JPEG and other supported raster clipboard formats continue through the separate bounded image decoder.

The existing 0.25.3 receiver handshake deadline remains in place. Pairings, permissions, the `seamlesscontrol/7` protocol, file transfers and screen crossing are unchanged. To preserve an SVG with embedded images exactly, transfer its original file using the approved Files flow.

**Update both Omarchy and Windows agents** when using image clipboard sharing across systems. On Omarchy, end the control session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, select **Update agent**, and restart **Receive control**. On Windows, exit from the tray, download the Windows x64 ZIP and adjacent SHA-256 file from this release, verify the ZIP, replace both executables together, and reopen the app.

Local regression tests check that a vector SVG still renders and that an embedded WebP is ignored. The full Rust suite, Clippy and Windows x64 build status are recorded in the [test record](TEST-RESULTS.md). This targeted fix has not yet had a new physical cross-system SVG test.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.4 · Imágenes incrustadas en SVG

Los SVG copiados al portapapeles ahora representan solo su contenido vectorial. Se omiten las imágenes incrustadas mediante URL `data:` o rutas, y el renderizador SVG se compila sin su decodificador de imágenes ráster. Esto cierra una vía por la que un WebP animado incrustado podía reservar mucha más memoria de la permitida por el límite del lienzo SVG. Los formatos WebP, PNG, JPEG y demás imágenes ráster admitidas siguen usando el decodificador independiente con límites.

Se conserva el plazo del handshake del receptor de la versión 0.25.3. No cambian emparejamientos, permisos, protocolo `seamlesscontrol/7`, archivos ni cruce de pantallas. Para conservar exactamente un SVG con imágenes incrustadas, transfiere su archivo original mediante el flujo aprobado de Archivos.

**Actualiza ambos agentes Omarchy y Windows** si compartes imágenes entre sistemas. En Omarchy, termina la sesión, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell`, pulsa **Actualizar agente** y reinicia **Recibir control**. En Windows, sal desde la bandeja, descarga de este lanzamiento el ZIP x64 y su archivo SHA-256, verifica el ZIP, sustituye juntos los dos ejecutables y abre la app.

Las pruebas locales de regresión comprueban que un SVG vectorial sigue representándose y que se omite un WebP incrustado. La suite Rust, Clippy y el estado de la compilación Windows x64 figuran en el [registro de pruebas](TEST-RESULTS.es.md). Esta corrección concreta aún no cuenta con una nueva prueba física de SVG entre sistemas.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
