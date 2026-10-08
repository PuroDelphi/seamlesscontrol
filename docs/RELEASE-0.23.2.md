# SeamlessControl 0.23.2 · Windows receiver connection limit

> **Historical release notes / Notas históricas.** Instructions below belong to this version. For current installation and use, see [English](../README.md) / [Español](../README.es.md).

This maintenance release addresses the marketplace review of 0.23.1. An unauthenticated LAN client could previously leave the Windows control receiver with an unbounded number of worker threads and open sockets by starting many connections that waited for the long pairing timeout.

The receiver now admits at most **eight concurrent control connections**; excess connections close before a thread is created. An initial Noise handshake has a ten-second I/O timeout. Once that handshake succeeds, the existing longer window remains available for people to compare and approve the six-digit pairing code. A Windows test covers the connection limit and slot release. Omarchy controls, saved pairings, and the regular connection flow are unchanged.

## Install or update

**Omarchy:** `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` installs the plugin; choose **Install agent** in its panel. To update, end active sessions, run `omarchy plugin update seamlesscontrol.control --yes`, restart the shell with `omarchy restart shell`, and choose **Update agent**. Both Omarchy computers should use the same current release.

**Windows x64:** exit the app from its tray menu. Under **Assets**, download `seamlesscontrol-windows-x64.zip` and its adjacent `.sha256`, extract the ZIP, and replace both executables together. Reopen `seamlesscontrol.exe`. Existing pairings and preferences remain saved.

[Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.2/docs/USER-GUIDE.md) · [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.2/docs/WINDOWS.md)

---

# SeamlessControl 0.23.2 · Límite de conexiones en el receptor Windows

Esta versión de mantenimiento atiende la revisión del marketplace de 0.23.1. Antes, un equipo de la LAN sin autenticar podía iniciar muchas conexiones que esperaban el plazo largo de emparejamiento y aumentar sin límite los hilos y sockets abiertos del receptor Windows.

Ahora el receptor admite como máximo **ocho conexiones de control simultáneas**; cierra las restantes antes de crear un hilo. El intercambio Noise inicial tiene un tiempo de espera de E/S de diez segundos. Cuando termina correctamente, se conserva el plazo más largo para que las personas comparen y aprueben el código de seis cifras. Una prueba Windows cubre el límite y la liberación de cupos. Los controles Omarchy, los emparejamientos guardados y el flujo habitual de conexión permanecen iguales.

## Instalar o actualizar

**Omarchy:** `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` instala el plugin; después pulsa **Instalar agente** en el panel. Para actualizar, termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia la shell con `omarchy restart shell` y pulsa **Actualizar agente**. Conviene que ambos Omarchy usen la misma versión actual.

**Windows x64:** sal de la app desde la bandeja. En **Assets**, descarga `seamlesscontrol-windows-x64.zip` y su `.sha256` contiguo, extrae el ZIP y reemplaza juntos ambos ejecutables. Abre otra vez `seamlesscontrol.exe`. Se conservan los emparejamientos y preferencias.

[Guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.2/docs/USER-GUIDE.es.md) · [Guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.2/docs/WINDOWS.es.md)
