# SeamlessControl 0.25.8 · Windows receiver protection

The marketplace reviewer found that the Windows control receiver limited itself to eight connection threads but relied on a timeout that restarted on every read of the Noise handshake. An unpaired client on the permitted private LAN could slowly send incomplete handshake bodies to occupy all eight slots and delay legitimate connections.

The Windows receiver now starts a **ten-second total deadline when each inbound socket takes a slot**. It closes the socket if the pairing preface and Noise handshake have not completed by then, regardless of how frequently bytes arrive. The deadline is cancelled immediately after Noise completes, before the six-digit pairing code may need human approval. The existing slot limit and longer authenticated-session timeouts remain. The file receiver already has its own total deadline; both synchronous receivers now reuse the same bounded transport helper. Omarchy's control receiver keeps its existing asynchronous deadline.

Saved pairings, the six-digit approval flow, peer permissions, clipboard, encrypted file transfer and protocol `seamlesscontrol/7` are compatible. **Omarchy update:** end an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and choose **Update agent** in Settings. **Windows update:** exit the tray app, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app. Updating a Windows receiver is necessary for this Windows-specific protection.

A Windows loopback regression test occupies all eight slots with incomplete Noise bodies sent in fragments, confirms they are released after the total deadline, and then establishes a real pinned connection. It delays the authenticated greeting beyond the test deadline to verify that a legitimate session remains usable after Noise. Linux tests, Clippy and Windows x64 CI are recorded in the [test results](TEST-RESULTS.md). This targeted fix has not had a new physical two-computer test.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.8 · Protección del receptor Windows

El revisor del marketplace encontró que el receptor de control Windows limitaba sus hilos de conexión a ocho, pero usaba una espera que empezaba de nuevo con cada lectura del saludo Noise. Un cliente sin emparejar de la LAN privada permitida podía enviar lentamente cuerpos incompletos, ocupar los ocho cupos y retrasar las conexiones legítimas.

El receptor Windows inicia ahora un **plazo total de diez segundos cuando cada socket entrante ocupa un cupo**. Cierra el socket si no terminan el prefacio de emparejamiento y Noise dentro de ese plazo, aunque sigan llegando bytes. El plazo se cancela justo al completar Noise, antes de que una persona tenga que aprobar el código de seis cifras. Se conservan el límite de cupos y los tiempos normales de la sesión autenticada. El receptor de archivos ya tenía su propio plazo total; ambos receptores síncronos reutilizan ahora el mismo auxiliar acotado. El receptor de control Omarchy conserva su plazo asíncrono.

Siguen siendo compatibles los emparejamientos guardados, la aprobación de seis cifras, permisos, portapapeles, transferencia cifrada y protocolo `seamlesscontrol/7`. **Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga el ZIP x64 y el SHA-256 contiguo de este lanzamiento, verifícalo, sustituye juntos los dos ejecutables y abre la app. Esta protección específica requiere actualizar el receptor Windows.

Una prueba de loopback Windows ocupa los ocho cupos con cuerpos Noise incompletos enviados en fragmentos, comprueba que se liberan al terminar el plazo total y después establece una conexión real con clave fijada. Retrasa el saludo autenticado más allá del plazo de prueba para comprobar que una sesión legítima sigue utilizable tras Noise. Las pruebas Linux, Clippy y CI Windows x64 constan en los [resultados](TEST-RESULTS.es.md). Esta corrección concreta no ha tenido una nueva prueba física entre dos equipos.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
