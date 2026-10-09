# SeamlessControl 0.23.3 · Discovery and runtime maintenance

SeamlessControl continues to connect Omarchy and Windows computers through shared mouse, keyboard, text clipboard and approved file copying. This release updates the locked Tokio runtime to 1.53.2 and `mdns-sd` discovery library to 0.21.5. The discovery update fixes timers after clock changes or system sleep. Application code and the pairing protocol are unchanged.

The English and Spanish guides are now organized around installation, first connection, return control, files, permissions and troubleshooting. Refreshed interface images use fictional data, and the guides explain direct-session layouts and the two Windows Connect steps more clearly.

## Install or update

**Omarchy:** install with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` and choose **Install agent** in the panel. To update, end active sessions, run `omarchy plugin update seamlesscontrol.control --yes`, restart the shell with `omarchy restart shell`, and choose **Update agent**. Update both Omarchy computers to keep their agents aligned.

**Windows x64:** exit SeamlessControl from the tray, download `seamlesscontrol-windows-x64.zip` and its adjacent `.sha256` from **Assets**, extract the ZIP and replace both executables together, then reopen `seamlesscontrol.exe`. Pairings and preferences remain saved.

[Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.3/docs/USER-GUIDE.md) · [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.3/docs/WINDOWS.md)

---

# SeamlessControl 0.23.3 · Mantenimiento del descubrimiento y las dependencias

SeamlessControl sigue conectando equipos Omarchy y Windows con un mismo ratón y teclado, portapapeles de texto y copia de archivos aprobada. Esta versión actualiza las dependencias fijadas Tokio a 1.53.2 y `mdns-sd` a 0.21.5. La actualización del descubrimiento corrige los temporizadores tras cambios de hora o suspensión. No cambia el código de la aplicación ni el protocolo de emparejamiento.

Las guías en español e inglés están organizadas por instalación, primera conexión, regreso, archivos, permisos y resolución de problemas. Las imágenes renovadas de la interfaz usan datos ficticios, y las guías aclaran el mapa para sesiones directas y los dos pasos de Conectar en Windows.

## Instalar o actualizar

**Omarchy:** instala con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` y pulsa **Instalar agente** en el panel. Para actualizar, termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia la shell con `omarchy restart shell` y pulsa **Actualizar agente**. Actualiza ambos Omarchy para mantener alineados los agentes.

**Windows x64:** sal de SeamlessControl desde la bandeja, descarga `seamlesscontrol-windows-x64.zip` y su `.sha256` contiguo en **Assets**, extrae el ZIP y reemplaza juntos ambos ejecutables; después abre `seamlesscontrol.exe`. Se conservan los emparejamientos y preferencias.

[Guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.3/docs/USER-GUIDE.es.md) · [Guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.3/docs/WINDOWS.es.md)
