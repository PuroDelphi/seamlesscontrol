# SeamlessControl 0.24.3 · Bounded, responsive discovery

Automatic discovery still finds nearby Omarchy and Windows computers, while pairing continues to require the matching six digit code and approval on both sides. This release closes the remaining resource issue identified during marketplace review: repeated announcements of a cached DNS record could keep adding timers, and a continuous packet stream could delay their cleanup.

The mDNS daemon now merges timer wakeups into 50 ms slots, reads a finite batch of packets from each socket before running expiry work, and revisits sockets with unread packets. It also limits delayed responses, expires orphan DNS records and removes per-service bookkeeping after a goodbye or expiry. The 0.24.2 limits of 64 nearby computers, 1,024 mDNS cache storage units and 120-second received TTL remain in force. The 0.24.1 Qt clipboard fix remains included. Pairing keys, encrypted protocol and saved settings are unchanged.

**Update Omarchy:** end active sessions, run `omarchy plugin update seamlesscontrol.control --yes` and `omarchy restart shell`, then choose **Update agent** in the panel. **Update Windows x64:** download this release's Windows ZIP and adjacent `.zip.sha256`, then use **Settings → Update from release ZIP…**. Update both Windows executables together.

Validation exercises 100,000 repeated timer refreshes, packet-drain yielding, orphan-record expiry and the earlier adversarial cache test. The vendored mDNS library passed 70 tests with its existing environment-sensitive `test_custom_port_isolation` excluded; that test also fails in this environment on the original, unmodified 0.21.5 source. Agent and Windows CI results are linked from the marketplace submission. No new physical cross-computer session is claimed for this security fix; the [0.24.0 physical results](TEST-RESULTS.md) remain available.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.24.3 · Descubrimiento estable y acotado

El descubrimiento automático sigue encontrando equipos Omarchy y Windows cercanos, mientras el emparejamiento exige el mismo código de seis cifras y aprobación en ambos lados. Esta versión corrige el recurso pendiente señalado durante la revisión del marketplace: los anuncios repetidos de un registro DNS almacenado podían seguir agregando temporizadores y un flujo continuo de paquetes podía retrasar su limpieza.

El agente mDNS ahora agrupa los temporizadores en intervalos de 50 ms, procesa un número finito de paquetes por socket antes de limpiar registros vencidos y vuelve a revisar los sockets con paquetes pendientes. También limita las respuestas aplazadas, elimina registros DNS huérfanos al vencer y libera el seguimiento de servicios que desaparecen. Se conservan los límites de 0.24.2: 64 equipos cercanos, 1.024 unidades de caché mDNS y TTL recibido máximo de 120 segundos. También se conserva la corrección del portapapeles Qt de 0.24.1. No cambian las claves de emparejamiento, el protocolo cifrado ni los ajustes guardados.

**Actualizar Omarchy:** termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y `omarchy restart shell`; después elige **Actualizar agente** en el panel. **Actualizar Windows x64:** descarga el ZIP Windows de este lanzamiento y el `.zip.sha256` contiguo; usa **Ajustes → Actualizar desde el ZIP de lanzamiento…**. Actualiza juntos ambos ejecutables.

Las pruebas ejercitan 100.000 renovaciones de temporizadores, la cesión de turno al procesar paquetes, la caducidad de registros huérfanos y la prueba adversarial anterior de caché. La biblioteca mDNS incluida pasó 70 pruebas omitiendo `test_custom_port_isolation`, que falla en este entorno también sobre el código original sin modificar de 0.21.5. Los resultados de CI del agente y Windows se enlazan en el envío al marketplace. No se afirma una nueva prueba física entre equipos para este arreglo; permanecen disponibles los [resultados físicos de 0.24.0](TEST-RESULTS.es.md).

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
