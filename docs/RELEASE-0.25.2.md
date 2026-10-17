# SeamlessControl 0.25.2 · Images in the clipboard and file drops into the app

> Historical release note: the Omarchy file-drop target was removed in [0.25.11](RELEASE-0.25.11.md) because Qt read unbounded foreign URI data. File-manager Copy and Paste remains available; Windows in-app drops remain available.

Copy an image in an application on one connected computer and paste it on the other. SeamlessControl accepts image clipboard data in PNG, JPEG, GIF, WebP, TIFF, BMP, ICO, DDS, OpenEXR, HDR, PNM, QOI, TGA, Farbfeld and SVG formats, plus compatible Windows bitmap clipboard representations. It converts the image to a bounded PNG for the authenticated control session. Animations use their first frame; high-depth images become 8-bit PNG. An image that is too large or cannot be decoded is ignored without interrupting mouse and keyboard control. To preserve an original image file in any format, send the file instead.

The **Files** page on Omarchy and Windows now accepts local files and folders dropped onto the sending app. This uses the existing file offer, peer Files permission, receiver approval mode, size limit, checksum verification and private staging. The source is copied, never moved. Dropping a file across the screen edge directly into the other computer's file manager is a separate feature and is not included in 0.25.2.

**Update both ends together.** The control protocol is `seamlesscontrol/7`; an older agent cannot share this session. On Omarchy, end active sessions, run `omarchy plugin update seamlesscontrol.control --yes` and `omarchy restart shell`, then choose **Update agent** in the panel on each computer. On Windows x64, download this release's `seamlesscontrol-windows-x64.zip` and adjacent `.zip.sha256` asset; choose **Settings → Update from release ZIP…**. The updater verifies the ZIP and replaces both executables. Pairings, layout and preferences are preserved.

Two physical Omarchy computers running 0.25.2 exchanged JPEG, SVG and WebP test images in both directions; the received dimensions and colors matched, a large image pasted visibly, and an image copied from an ordinary application pasted on the other computer. The Linux suite and Windows x64 CI passed. Physical Windows image paste and app file drops remain to be checked. The [test record](TEST-RESULTS.md) describes the evidence and its scope.

[Omarchy guide](USER-GUIDE.md) · [Windows guide](WINDOWS.md) · [Technical guide](TECHNICAL.md)

---

# SeamlessControl 0.25.2 · Imágenes en el portapapeles y archivos arrastrados a la app

> Nota histórica: la zona de archivos soltados del panel Omarchy se retiró en [0.25.11](RELEASE-0.25.11.md) porque Qt leía URI ajenas sin límite. Sigue disponible Copiar y Pegar; la app Windows conserva el arrastre a su página Archivos.

Copia una imagen en una aplicación de un equipo conectado y pégala en el otro. SeamlessControl admite datos de imagen PNG, JPEG, GIF, WebP, TIFF, BMP, ICO, DDS, OpenEXR, HDR, PNM, QOI, TGA, Farbfeld y SVG, además de mapas de bits compatibles del portapapeles Windows. La convierte a un PNG acotado para la sesión de control autenticada. De las animaciones usa el primer fotograma; las imágenes de alta profundidad pasan a PNG de 8 bits. Una imagen demasiado grande o incompatible se ignora sin interrumpir el ratón ni el teclado. Para conservar el archivo original de una imagen de cualquier formato, envíalo como archivo.

La página **Archivos** de Omarchy y Windows acepta archivos y carpetas locales soltados en la app emisora. Utiliza la oferta de archivos existente, el permiso Archivos del par, la aprobación del receptor, el límite de tamaño, la verificación de checksum y la carpeta temporal privada. Copia el origen, nunca lo mueve. Soltar un archivo a través del borde directamente en el explorador del otro equipo es una función distinta que no forma parte de 0.25.2.

**Actualiza ambos extremos juntos.** El protocolo de control es `seamlesscontrol/7`; un agente antiguo no puede compartir esta sesión. En Omarchy, termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y `omarchy restart shell`; después pulsa **Actualizar agente** en el panel de cada equipo. En Windows x64, descarga los assets `seamlesscontrol-windows-x64.zip` y `.zip.sha256` contiguo de este lanzamiento; usa **Ajustes → Actualizar desde el ZIP de lanzamiento…**. El actualizador verifica el ZIP y reemplaza ambos ejecutables. Se conservan los emparejamientos, el mapa y las preferencias.

Dos Omarchy físicos con 0.25.2 intercambiaron imágenes de prueba JPEG, SVG y WebP en ambos sentidos; coincidieron dimensiones y colores, una imagen grande se pegó visiblemente y una imagen copiada desde una aplicación normal se pegó en el otro equipo. Pasaron la suite Linux y CI de Windows x64. Quedan por comprobar físicamente el pegado de imágenes en Windows y los archivos soltados en la app. El [registro de pruebas](TEST-RESULTS.es.md) delimita los resultados.

[Guía Omarchy](USER-GUIDE.es.md) · [Guía Windows](WINDOWS.es.md) · [Guía técnica](TECHNICAL.es.md)
