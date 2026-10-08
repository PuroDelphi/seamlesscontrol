# SeamlessControl 0.22.0 · Omarchy and Windows together

> **Historical release notes / Notas históricas.** Instructions below belong to this version. For current installation and use, see [English](../README.md) / [Español](../README.es.md).

Move the pointer between Omarchy and Windows and keep working with the same keyboard, text clipboard and files. This release brings the simpler interface and the new copied-file experience to the published version.

## Highlights

- **Clearer everyday controls.** The Omarchy panel separates Home, Computers, Files and Settings, with optional help collapsed until needed. The Windows tray app uses the same task flow, puts paired computers beside its draggable map and remembers your last connection.
- **Copy here, paste there.** Copy one file in the Omarchy file manager or Windows Explorer. The paired receiver shows an actionable offer, verifies the accepted file and makes it available to Paste in your chosen folder. The prompt reaches the active Omarchy workspace or Windows virtual desktop even with the main window closed.
- **Approval on your terms.** Ask for every file, accept authenticated files automatically from paired computers, or approve the first file and accept that computer's later files for a chosen time. Both interfaces confirm saved settings, show the temporary countdown and return to **Ask every time** when it ends.
- **One Windows download.** Under **Assets**, download `seamlesscontrol-windows-x64.zip`; it contains both `seamlesscontrol.exe` and `seamlesscontrold.exe`. The adjacent `.sha256` verifies the complete ZIP. Windows test builds in Actions have one downloadable artifact with both executables and their hashes inside.

Physical Omarchy/Windows tests confirmed control and return in both directions, text clipboard in both directions, copied-file approval and Paste in both directions, rejection without transfer, temporary and automatic approval, Unicode filenames and duplicate copies. The [test record](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/TEST-RESULTS.md) lists scenarios that still need more computers or fault tests.

## Install or update

On Omarchy, install with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, then choose **Install agent** in the panel. For an existing installation, stop active sessions, run `omarchy plugin update seamlesscontrol.control`, restart the Omarchy shell and choose **Update agent** in the panel. Pairing keys and layout stay saved.

On Windows, exit SeamlessControl from the tray, extract the new Windows ZIP over the two executables in your chosen folder, and reopen `seamlesscontrol.exe`. Your pairing data remains under `%LOCALAPPDATA%\SeamlessControl`.

The receiver uses TCP `47832` for control, TCP `47833` for manual file transfers, and TCP `47834` for copied files; mDNS discovery uses UDP `5353`. The interfaces guide users through private-LAN firewall authorization. See the [Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/USER-GUIDE.md), [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/WINDOWS.md), and [technical guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/TECHNICAL.md).

---

# SeamlessControl 0.22.0 · Omarchy y Windows juntos

Cruza con el puntero entre Omarchy y Windows y sigue trabajando con el mismo teclado, portapapeles de texto y archivos. Esta versión lleva la interfaz simplificada y la nueva copia de archivos a la edición publicada.

## Novedades

- **Controles claros para el uso diario.** El panel Omarchy separa Inicio, Equipos, Archivos y Ajustes, y deja plegada la ayuda opcional. La app Windows de bandeja sigue el mismo flujo, sitúa los equipos emparejados junto a su mapa arrastrable y recuerda la última conexión.
- **Copia aquí, pega allá.** Copia un archivo en el explorador de Omarchy o Windows. El receptor emparejado muestra una oferta visible, verifica el archivo aceptado y permite Pegar en la carpeta elegida. El aviso llega al workspace activo de Omarchy o al escritorio virtual de Windows aunque la ventana principal esté cerrada.
- **Aprobación a tu medida.** Pregunta por cada archivo, acepta automáticamente archivos autenticados de equipos emparejados o aprueba el primero y recibe los siguientes de ese equipo durante los minutos elegidos. Ambas interfaces confirman el guardado, muestran la cuenta regresiva y vuelven a **Preguntar siempre** al vencer.
- **Una descarga para Windows.** En **Assets**, descarga `seamlesscontrol-windows-x64.zip`; contiene `seamlesscontrol.exe` y `seamlesscontrold.exe`. El `.sha256` contiguo verifica el ZIP completo. Las compilaciones de prueba en Actions ofrecen un solo artefacto con ambos ejecutables y sus sumas dentro.

Las pruebas físicas entre Omarchy y Windows confirmaron control y regreso en ambos sentidos, portapapeles de texto en ambos sentidos, aprobación y pegado de archivos copiados en ambos sentidos, rechazo sin transferencia, aprobación temporal y automática, nombres Unicode y copias duplicadas. El [registro de pruebas](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/TEST-RESULTS.es.md) indica los casos que aún requieren más equipos o pruebas de fallos.

## Instalar o actualizar

En Omarchy, instala con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` y pulsa **Instalar agente** en el panel. Si ya lo tienes, termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control`, reinicia el shell de Omarchy y pulsa **Actualizar agente**. Se conservan las claves emparejadas y el mapa.

En Windows, sal de SeamlessControl desde la bandeja, extrae el ZIP nuevo sobre ambos ejecutables en la carpeta elegida y vuelve a abrir `seamlesscontrol.exe`. Los emparejamientos permanecen en `%LOCALAPPDATA%\SeamlessControl`.

El receptor usa TCP `47832` para control, TCP `47833` para envíos manuales y TCP `47834` para archivos copiados; el descubrimiento mDNS usa UDP `5353`. Las interfaces guían la autorización del firewall limitada a la LAN privada. Consulta la [guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/USER-GUIDE.es.md), la [guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/WINDOWS.es.md) y la [guía técnica](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.22.0/docs/TECHNICAL.es.md).
