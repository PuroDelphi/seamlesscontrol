# SeamlessControl 0.25.13 · Bind control to the authenticated peer

The marketplace reviewer found that the Windows console receiver could authenticate one peer, wait for its control claim, and then read a **different peer's** newly paired key from the same IP. This release keeps the authenticated Noise public key with that connection and checks the current pin, revocation and control policy against that key before input starts and while the session runs. The Omarchy receiver now performs the same live checks.

- A pending old connection cannot inherit control permission after its IP is revoked and paired to another identity. It is rejected before `READY` and cannot inject input.
- If a live peer is revoked, replaced or loses control or text permission, its receiver socket closes and held input is released. A newly paired computer must make its own fresh authenticated connection.
- The normal pairing flow, existing trusted peer records and wire protocol remain unchanged.

The Windows regression test establishes a real encrypted connection using the old identity, replaces its pin while the connection waits for `CLAIM`, and verifies that the original key remains attached and is denied. A Linux test checks the same pin replacement rule. Linux tests and Windows x64 CI are recorded in the [test results](TEST-RESULTS.md). This focused security change has not had a new physical two-computer test.

**Update both computers.** On Omarchy, end the active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and select **Update agent** in Settings. On Windows, exit from the tray, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.13 · Identidad vinculada a la conexión

El revisor del marketplace encontró que el receptor de consola de Windows podía autenticar a un equipo, esperar su solicitud de control y después leer la clave recién emparejada de **otro equipo** en la misma IP. Esta versión conserva la clave pública autenticada por Noise durante toda la conexión y comprueba la clave emparejada, la revocación y los permisos contra esa identidad antes y durante el control. El receptor Omarchy hace ahora las mismas comprobaciones.

- Una conexión antigua en espera no puede heredar permisos después de revocar su IP y emparejar otra identidad. Se rechaza antes de `READY` y no puede inyectar entrada.
- Si se revoca o reemplaza un equipo activo, o pierde permisos de control o texto, se cierra su conexión y se liberan las teclas y botones retenidos. El equipo nuevo debe iniciar su propia conexión autenticada.
- Se conservan el proceso normal de emparejamiento, los equipos ya emparejados y el protocolo de intercambio.

La prueba de Windows establece una conexión cifrada real con la identidad anterior, reemplaza su clave mientras espera `CLAIM` y verifica que conserva la clave original y se deniega. Una prueba Linux comprueba la misma sustitución. Las pruebas Linux y la compilación x64 de Windows constan en el [registro](TEST-RESULTS.es.md). No se hizo una nueva prueba física entre dos equipos para esta corrección concreta.

**Actualiza ambos equipos.** En Omarchy, termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. En Windows, sal desde la bandeja, descarga el ZIP x64 de este lanzamiento y su SHA-256 contiguo, verifícalo, sustituye juntos ambos ejecutables y abre la app.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
