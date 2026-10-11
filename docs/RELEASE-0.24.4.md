# SeamlessControl 0.24.4 · Active Windows sessions honor revocation

Revoking a paired computer on Windows now closes its current control session, including when revocation comes from the console command `seamlesscontrold.exe revoke <IP>`. Previously that command removed the saved key but a receiver process could keep an already authenticated channel open. It could continue accepting mouse, keyboard and enabled text clipboard events until the session ended for another reason.

The Windows receiver now checks the pinned identity and Control/Text permissions throughout the session. A separate short-interval watcher closes an idle or busy socket when its trust changes; the input receiver releases held keys and buttons. A Windows source also stops capture and returns local control when its peer is revoked or the permissions change. File transfer checks the pinned key and Files permission throughout a transfer, so a still-present control lease cannot authorize later chunks after revocation. The console command waits for any active session to close before reporting success, and reports an error if an older running receiver does not stop. Revocation still requires a fresh, mutually approved pairing code before reconnection.

**Update Windows x64:** download this release's Windows ZIP and adjacent `.zip.sha256`; in the app choose **Settings → Update from release ZIP…**. Both executables must be updated together. If using the console, exit and restart the old receiver after replacing both files. **Update Omarchy:** run `omarchy plugin update seamlesscontrol.control --yes`, `omarchy restart shell`, then **Update agent** in the panel.

Windows CI tests revocation against an open loopback receiver socket, checks that it closes, and sends input before revocation to verify a held key and mouse button are released. It also checks that Control/Text permission changes invalidate a live session and that revocation blocks files while a control lease still exists. Linux and Windows builds passed; no new physical cross-computer session is claimed for this targeted security fix. The 0.24.1 clipboard, 0.24.2 cache and 0.24.3 timer fixes remain included.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.24.4 · La revocación cierra las sesiones Windows

Revocar un equipo emparejado en Windows ahora cierra su sesión de control actual, incluso cuando se hace desde la consola con `seamlesscontrold.exe revoke <IP>`. Antes, el comando quitaba la clave guardada, pero el receptor podía conservar un canal ya autenticado. Así podía seguir aceptando ratón, teclado y portapapeles de texto habilitado hasta que la sesión terminara por otra causa.

El receptor Windows comprueba durante la sesión que la identidad sigue fijada y que los permisos Control y Texto no han cambiado. Un vigilante de intervalo corto cierra el socket aunque el equipo esté inactivo o enviando datos; el receptor libera teclas y botones retenidos. Si Windows es el origen, también detiene la captura y devuelve el control local al revocar o modificar estos permisos. Las transferencias verifican la clave fijada y el permiso Archivos durante el envío, de modo que un comprobante de control aún presente no autoriza partes posteriores tras revocar. El comando de consola espera a que termine cualquier sesión activa antes de confirmar éxito y muestra un error si un receptor antiguo sigue ejecutándose. Para reconectar tras revocar se requiere aprobar de nuevo un código coincidente en ambos equipos.

**Actualizar Windows x64:** descarga el ZIP Windows del lanzamiento y su `.zip.sha256` contiguo; en la app usa **Ajustes → Actualizar desde el ZIP de lanzamiento…**. Actualiza juntos los dos ejecutables. Si usas la consola, cierra el receptor antiguo y reinícialo después de sustituirlos. **Actualizar Omarchy:** ejecuta `omarchy plugin update seamlesscontrol.control --yes`, `omarchy restart shell` y después **Actualizar agente** en el panel.

CI de Windows prueba la revocación durante una conexión TCP de recepción abierta, comprueba el cierre del socket y envía entrada antes de revocar para verificar que se liberan una tecla y un botón retenidos. También comprueba que los cambios de permisos Control/Texto invalidan la sesión y que la revocación bloquea archivos aunque aún exista un comprobante de control. Pasaron las compilaciones Linux y Windows; no se afirma una nueva sesión física entre equipos para esta corrección específica. Se conservan los arreglos del portapapeles 0.24.1, caché 0.24.2 y temporizadores 0.24.3.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
