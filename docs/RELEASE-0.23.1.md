# SeamlessControl 0.23.1 · Clearer controls and safer Omarchy sessions

SeamlessControl brings Omarchy and Windows screens together with one mouse and keyboard, text clipboard synchronization, and approved file copying. This release makes the Omarchy panel easier to read and protects the source computer during long control sessions.

- **Clear button states.** Unavailable actions are visibly dimmed. **Update agent** explains when an active session prevents an update.
- **Automatic idle protection.** While one Omarchy controls another, its panel keeps the source awake without changing the user's permanent stay-awake preference. The agent also releases source capture when a lock is detected and waits for valid monitor geometry before restoring capture barriers after display sleep.
- **More reliable panel installation.** The installer selects Cargo and rustc from the same project context even when started from the Omarchy panel.
- **Keyboard diagnostics.** Optional input tracing can identify where an intermittent remote key stops. Normal sessions do not enable this trace.

In a physical two-Omarchy test, remote key presses reached both computers and the source stayed unlocked after more than five minutes without physical input while controlling the receiver. The physical mouse and keyboard worked after the test ended. The intermittent keyboard failure did not occur during this observation. Earlier deliberate lock tests froze source input; manual locking with this revision still needs a physical retest. See the [test record](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.1/docs/TEST-RESULTS.md).

## Install or update

**Omarchy:** install with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, then choose **Install agent** in the panel. To update, end active sessions, run `omarchy plugin update seamlesscontrol.control --yes`, then `omarchy restart shell` and choose **Update agent**. Update both Omarchy computers for this release. Pairing identities and screen layout remain saved.

**Windows x64:** exit SeamlessControl from its tray menu. Under **Assets**, download the single `seamlesscontrol-windows-x64.zip` and its adjacent `.sha256`. Extract the ZIP and replace both `seamlesscontrol.exe` and `seamlesscontrold.exe` together, then reopen the app. The release also has a tagged source archive and its adjacent checksum.

[Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.1/docs/USER-GUIDE.md) · [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.1/docs/WINDOWS.md)

---

# SeamlessControl 0.23.1 · Controles más claros y sesiones Omarchy más seguras

SeamlessControl reúne pantallas Omarchy y Windows con un mismo ratón y teclado, sincronización del portapapeles de texto y copia de archivos con aprobación. Esta versión aclara el panel Omarchy y protege el equipo de origen durante las sesiones largas.

- **Estados visibles.** Las acciones no disponibles se atenúan. **Actualizar agente** explica cuándo una sesión activa impide la actualización.
- **Protección frente al bloqueo automático.** Mientras un Omarchy controla otro, el panel mantiene despierto el origen sin cambiar la preferencia permanente del usuario. El agente libera la captura si detecta un bloqueo y espera a que la geometría del monitor sea válida antes de restaurar las barreras tras apagarse la pantalla.
- **Instalación más fiable desde el panel.** El instalador selecciona Cargo y rustc en el mismo contexto del proyecto, aunque se inicie desde el panel Omarchy.
- **Diagnóstico del teclado.** Un registro de entrada opcional permite localizar dónde se pierde una tecla remota intermitente. Las sesiones normales no lo activan.

En una prueba física entre dos Omarchy, las teclas remotas llegaron a ambos equipos y el origen permaneció desbloqueado tras más de cinco minutos sin actividad física mientras controlaba el receptor. Al terminar, funcionaban su ratón y teclado físicos. El fallo intermitente del teclado no apareció durante esa observación. Las pruebas anteriores de bloqueo deliberado congelaron la entrada del origen; aún hace falta repetir físicamente el bloqueo manual con esta revisión. Consulta el [registro de pruebas](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.1/docs/TEST-RESULTS.es.md).

## Instalar o actualizar

**Omarchy:** instala con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` y pulsa **Instalar agente** en el panel. Para actualizar, termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, luego `omarchy restart shell` y pulsa **Actualizar agente**. Actualiza ambos Omarchy para esta versión. Se conservan las identidades y posiciones de pantalla.

**Windows x64:** sal de SeamlessControl desde el icono de bandeja. En **Assets**, descarga el único `seamlesscontrol-windows-x64.zip` y su `.sha256` contiguo. Extrae el ZIP y reemplaza juntos `seamlesscontrol.exe` y `seamlesscontrold.exe`; después vuelve a abrir la app. El release también incluye un archivo del código fuente etiquetado y su suma de verificación.

[Guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.1/docs/USER-GUIDE.es.md) · [Guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.1/docs/WINDOWS.es.md)
