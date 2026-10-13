# SeamlessControl 0.25.3 · Bounded authentication on the Omarchy receiver

The Omarchy receiver now allows at most ten seconds for an incoming connection to complete its optional pairing preface and Noise handshake. A computer that has not authenticated can no longer keep one of the eight incoming connection slots occupied for hours by slowly sending only part of a handshake. The connection is closed and its slot is released when that deadline expires.

The ten-second clock ends **before** a person is asked to compare and approve the six-digit pairing code. The normal time to approve a new pairing is unchanged. Existing peer keys, permissions, protocol `seamlesscontrol/7`, Windows compatibility, file transfers and screen crossing do not change. The Windows app does not need a new binary for this Linux receiver fix.

**Update the Omarchy receiver:** end its active control session, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, then choose **Update agent** in the panel. Restart **Receive control** so the new agent process listens on the control port. Saved pairings and screen layout remain in place.

Two local loopback regression tests confirm that an incomplete Noise body releases a connection slot and that a completed handshake cancels the deadline. The full Rust suite and Clippy pass. This fix has not yet had a new physical two-computer denial-of-service test; the [test record](TEST-RESULTS.md) separates that from the earlier physical control and image tests.

The illustrated Omarchy and Windows guides have also had their demonstration banners removed. Example network addresses in the images remain fictional.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.3 · Plazo de autenticación en el receptor Omarchy

El receptor Omarchy permite ahora un máximo de diez segundos para completar el prefacio opcional de emparejamiento y el handshake Noise de una conexión entrante. Un equipo que todavía no se autenticó ya no puede ocupar durante horas uno de los ocho cupos enviando lentamente solo una parte del handshake. Al vencer el plazo, se cierra la conexión y se libera el cupo.

El reloj de diez segundos termina **antes** de pedir a una persona que compare y apruebe el código de seis cifras. El tiempo normal para aprobar un emparejamiento nuevo no cambia. Tampoco cambian las claves y permisos guardados, el protocolo `seamlesscontrol/7`, la compatibilidad con Windows, la transferencia de archivos ni el cruce entre pantallas. Esta corrección del receptor Linux no exige un ejecutable Windows nuevo.

**Actualizar el receptor Omarchy:** termina su sesión de control activa, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente**. Reinicia **Recibir control** para que el proceso nuevo escuche en el puerto de control. Se conservan los emparejamientos y el mapa de pantallas.

Dos pruebas locales de regresión comprueban que un cuerpo Noise incompleto libera un cupo y que un handshake completado cancela el plazo. Pasan toda la suite Rust y Clippy. Esta corrección aún no cuenta con una prueba física nueva de denegación de servicio entre dos equipos; el [registro de pruebas](TEST-RESULTS.es.md) la distingue de las pruebas físicas anteriores de control e imágenes.

También se retiraron las bandas de demostración de las guías ilustradas Omarchy y Windows. Las direcciones de red de ejemplo en las imágenes siguen siendo ficticias.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
