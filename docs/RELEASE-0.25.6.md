# SeamlessControl 0.25.6 · File receiver protection

The marketplace reviewer found that the one-shot file receiver could remain occupied before authentication. Its five-second timeout restarted on every read, so an unpaired process on an already paired computer could send an incomplete Noise message slowly and delay a legitimate file offer for much longer.

The shared Omarchy and Windows file receiver now closes a connection unless Noise authentication **and** the encrypted file greeting finish within ten seconds total. A legitimate client can then use the receiver. The deadline ends before an offer waits for your approval, so the file confirmation flow is unchanged. Saved pairings, permissions, file limits, encrypted transfer and protocol `seamlesscontrol/7` remain compatible.

**Omarchy update:** end an active session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, and choose **Update agent** in Settings. **Windows update:** exit the tray app, download this release's Windows x64 ZIP and adjacent SHA-256 file, verify it, replace both executables together, and reopen the app.

A local socket regression test holds an incomplete handshake open by trickling bytes and confirms that the receiver admits a legitimate encrypted transfer after the total deadline. Full Rust tests, Clippy and Windows x64 CI are recorded in the [test results](TEST-RESULTS.md). This targeted fix has not had a new physical two-computer file-transfer test.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.6 · Protección del receptor de archivos

El revisor del marketplace encontró que el receptor de archivos podía quedar ocupado antes de autenticar al remitente. Su espera de cinco segundos empezaba de nuevo en cada lectura, así que un proceso sin emparejar en un equipo previamente emparejado podía enviar lentamente un mensaje Noise incompleto y retrasar durante mucho tiempo una oferta legítima.

El receptor compartido por Omarchy y Windows cierra ahora la conexión si no terminan la autenticación Noise **y** el saludo cifrado de archivos en diez segundos en total. Después puede atender a un cliente legítimo. El plazo termina antes de esperar la aprobación del archivo. Se conservan emparejamientos, permisos, límites, cifrado y protocolo `seamlesscontrol/7`.

**Actualizar Omarchy:** termina la sesión activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente** en Ajustes. **Actualizar Windows:** sal desde la bandeja, descarga el ZIP x64 y el archivo SHA-256 contiguo de este lanzamiento, verifícalo, sustituye juntos los dos ejecutables y abre la app.

Una prueba local de sockets mantiene incompleto el saludo mediante fragmentos y comprueba que, al vencer el plazo total, el receptor admite una transferencia legítima cifrada. Las pruebas Rust, Clippy y la CI Windows x64 constan en los [resultados](TEST-RESULTS.es.md). Esta corrección concreta aún no se ha probado de nuevo con una transferencia entre dos equipos físicos.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
