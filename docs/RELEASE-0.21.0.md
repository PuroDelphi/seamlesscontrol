# SeamlessControl 0.21.0 · Omarchy meets Windows

> **Historical release notes / Notas históricas.** Instructions below belong to this version. For current installation and use, see [English](../README.md) / [Español](../README.es.md).

Move naturally between Omarchy and Windows with one physical mouse and keyboard. Cross the edge of one screen to control the next computer, then cross back or press Escape. This release brings the Windows x64 tray app into the same SeamlessControl ecosystem as the Omarchy plugin.

## What you can do

- **Control in both directions.** Omarchy can control Windows, and Windows can control Omarchy. The user confirmed pointer movement, clicks, keyboard input, tested shortcuts, edge return and Escape return on physical Omarchy and Windows 11 x64 computers.
- **Find computers automatically.** Both apps discover available receivers on the private LAN. The Windows announcement was physically observed in the Omarchy panel. Pairing still requires matching and approving the six digit code on both computers.
- **Keep text at hand.** Text clipboard synchronization between Omarchy and Windows was confirmed in both directions.
- **Send an approved file.** The receiver reviews each offer, and the sender verifies delivery. An Omarchy-to-Windows file transfer was confirmed on physical computers; transfer uses its own LAN port.
- **Use a visual layout.** The Omarchy panel and Windows app both place paired computers around the local screen. On Windows, paired computers appear directly above the map with a drag guide and keyboard controls.
- **Leave Windows running in the tray.** The bilingual Windows app keeps receiving when its window closes and reopens from the tray. Its executable now carries the same icon as the tray.

The [preview](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.0/preview.png) shows the shared Omarchy and Windows workspace. We will keep improving SeamlessControl and expanding the verified computer combinations. The [test record](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.0/docs/TEST-RESULTS.md) distinguishes observed physical behavior from additional scenarios such as multi-computer mesh.

## Install or update Omarchy

Install the plugin with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, open its panel and select **Install agent**. To update an existing installation, stop its sessions, run `omarchy plugin update seamlesscontrol.control`, run `omarchy restart shell`, then choose **Update agent** in the panel. Pairing keys and screen layout are kept. See the [Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.0/docs/USER-GUIDE.md).

## Install or update Windows x64

Under **Assets** below, download **both** `seamlesscontrol.exe` and `seamlesscontrold.exe` and keep them in the same folder. Their adjacent `.sha256` files contain the SHA-256 digest of each executable. Open `seamlesscontrol.exe`; it starts receiving and can stay in the tray. To update, exit from the tray, replace both executables and open the app again. Your local identity and pairings under `%LOCALAPPDATA%\SeamlessControl` remain in place. See the [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.0/docs/WINDOWS-ALPHA.md).

The source archive and its `.sha256` are also attached. The Omarchy installer builds from the plugin checkout with a locked dependency file; it does not download the Windows executables.

---

# SeamlessControl 0.21.0 · Omarchy se une a Windows

Cruza de Omarchy a Windows y vuelve con un solo ratón y teclado físicos. Esta versión incorpora la aplicación Windows x64 de bandeja al mismo ecosistema del plugin Omarchy.

- **Control en ambos sentidos:** se comprobaron físicamente Omarchy→Windows y Windows→Omarchy, con puntero, clics, teclado, atajos probados y regreso por borde o Escape.
- **Descubrimiento automático:** Windows apareció en el panel Omarchy sin escribir su IP. El emparejamiento sigue requiriendo comparar y aprobar el código de seis cifras en ambos equipos.
- **Portapapeles de texto:** sincronización comprobada en ambos sentidos.
- **Archivos con aprobación:** el receptor autoriza cada oferta y se verifica la entrega. Se comprobó físicamente un envío Omarchy→Windows.
- **Mapa visual y app de bandeja:** ubica los equipos arrastrando o con el teclado; la app Windows permanece disponible al cerrar su ventana y ahora muestra su propio icono en el ejecutable.

En **Assets**, descarga **ambos** `.exe` Windows x64 y consulta sus archivos `.sha256` contiguos. Para instalar en Omarchy, usa `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` y pulsa **Instalar agente** en el panel. Las instrucciones completas de actualización están en la [guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.0/docs/USER-GUIDE.es.md) y la [guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.0/docs/WINDOWS-ALPHA.es.md). Seguiremos mejorando SeamlessControl continuamente.
