# SeamlessControl 0.25.11 · Clipboard and panel security update

The marketplace review found two resource paths outside the previous SVG render budgets and file validation. This release closes both before their expensive work can start.

- SVG clipboard input is scanned before usvg builds its retained scene. `<use>` references and markers are rejected before they can multiply path geometry. The scan also caps text, geometry and attribute data, along with the existing XML source size, element count and depth limits. Simple vector SVG and bounded raster clipboard formats remain supported.
- The Omarchy panel no longer reads file drops from external drag sources. Qt fetched the foreign URI stream synchronously before SeamlessControl could check its 256-entry limit, which could stall the shared shell. On Omarchy, copy files or folders in the file manager, or use **Files → Choose** for a manual send. The Windows app still accepts local files dropped onto its Files page. All transfers retain the active-session, peer permission, approval, size and verification rules.

The layout drag within SeamlessControl still works: it reads only the app-owned source item. Pairings and protocol `seamlesscontrol/7` are unchanged.

**Update Omarchy:** end an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and select **Update agent** in Settings. **Update Windows:** exit from the tray, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app. Update both agents for SVG clipboard protection.

Regression tests cover repeated `<use>` references, markers, oversized text and path data, plus a static check that the Omarchy panel does not materialize `drop.urls`. Existing SVG and file transfer tests continue to run in CI. See the [test record](TEST-RESULTS.md). No new physical two-computer test was performed for this targeted security fix.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.11 · Seguridad del portapapeles y del panel

La revisión del marketplace encontró dos rutas de recursos fuera de los límites previos de SVG y de la validación de archivos. Este lanzamiento cierra ambas antes de iniciar el trabajo costoso.

- Los SVG del portapapeles se inspeccionan antes de que usvg construya su escena. Se rechazan referencias `<use>` y marcadores antes de que multipliquen la geometría. La inspección también limita texto, geometría y atributos, además de los límites existentes de tamaño, elementos y profundidad del XML. Siguen disponibles los SVG vectoriales sencillos y los formatos ráster acotados.
- El panel Omarchy deja de leer archivos soltados desde orígenes externos. Qt obtenía de forma síncrona el flujo de URI ajeno antes de que SeamlessControl pudiera comprobar el límite de 256 entradas, y podía bloquear el shell compartido. En Omarchy, copia archivos o carpetas en el explorador, o usa **Archivos → Elegir** para un envío manual. La app Windows sigue aceptando archivos locales soltados en su página Archivos. Todas las transferencias conservan las reglas de sesión activa, permisos, aprobación, tamaño y verificación.

El arrastre interno del mapa de equipos sigue funcionando: solo lee el elemento de origen de la propia app. Se mantienen los emparejamientos y el protocolo `seamlesscontrol/7`.

**Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga el ZIP x64 de este lanzamiento y su SHA-256 contiguo, verifícalo, sustituye juntos ambos ejecutables y vuelve a abrir la app. Actualiza ambos agentes para proteger el portapapeles SVG.

Las pruebas cubren referencias `<use>` repetidas, marcadores, texto y trazados excesivos, y una comprobación estática que impide volver a leer `drop.urls` en el panel Omarchy. Las pruebas anteriores de SVG y transferencias siguen en CI. Consulta el [registro de pruebas](TEST-RESULTS.es.md). No se realizó una nueva prueba física entre dos equipos para esta corrección puntual de seguridad.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
