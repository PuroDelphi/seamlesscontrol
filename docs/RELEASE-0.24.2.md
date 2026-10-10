# SeamlessControl 0.24.2 · Bounded discovery on Windows

SeamlessControl continues to discover nearby Omarchy and Windows computers automatically. This security update limits what an unauthenticated LAN advertiser can make the Windows app retain or redraw: at most 64 nearby computers in the panel, at most 1,024 storage units in the mDNS daemon's record cache, and no received record kept for more than 120 seconds without refresh. The panel publishes discovery changes at most twice per second. A peer still needs the matching six digit code and explicit approval on both computers before it can be paired.

The `mdns-sd` 0.21.5 source is vendored with a small, documented cache patch because its published release does not provide a record-count limit. The previous 0.24.1 Omarchy clipboard fix is included. No pairing or encrypted control protocol change is required.

**Update Omarchy:** end active sessions, run `omarchy plugin update seamlesscontrol.control --yes` and `omarchy restart shell`, then choose **Update agent** in the panel. **Update Windows x64:** download the release ZIP and its adjacent `.zip.sha256` file, then use **Settings → Update from release ZIP…**. Update both Windows executables together. Pairings and settings remain in place.

The adversarial test injects 5,000 distinct maximum-TTL records into the cache and verifies the bound; Windows CI also builds the app and exercises its nearby-list limit. Physical cross-computer behavior previously verified for 0.24.0 is recorded separately in the [test results](TEST-RESULTS.md).

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.24.2 · Descubrimiento acotado en Windows

SeamlessControl sigue descubriendo automáticamente equipos Omarchy y Windows cercanos. Esta actualización de seguridad limita lo que un anunciante de la red local sin autenticar puede hacer que la app Windows conserve o redibuje: hasta 64 equipos cercanos en el panel, hasta 1.024 unidades de almacenamiento en la caché mDNS y un máximo de 120 segundos para cada registro recibido sin renovarlo. El panel publica cambios de descubrimiento como máximo dos veces por segundo. Un equipo todavía necesita el código coincidente de seis cifras y la aprobación explícita en ambos lados para emparejarse.

Se incluye el código fuente de `mdns-sd` 0.21.5 con una pequeña modificación documentada porque la versión publicada no ofrece un límite de cantidad de registros. Se conserva el arreglo del portapapeles Omarchy de 0.24.1. El protocolo de emparejamiento y control cifrado no cambia.

**Actualizar Omarchy:** termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y `omarchy restart shell`, luego elige **Actualizar agente** en el panel. **Actualizar Windows x64:** descarga el ZIP del lanzamiento y su `.zip.sha256` contiguo, y usa **Ajustes → Actualizar desde el ZIP de lanzamiento…**. Actualiza juntos ambos ejecutables de Windows. Se conservan emparejamientos y ajustes.

La prueba adversarial introduce 5.000 registros distintos con TTL máximo en la caché y verifica el límite; CI de Windows también compila la app y prueba el límite de equipos cercanos. Las pruebas físicas entre equipos hechas para 0.24.0 se documentan aparte en el [registro de pruebas](TEST-RESULTS.es.md).

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.md)
