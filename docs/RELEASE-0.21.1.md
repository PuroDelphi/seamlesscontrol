# SeamlessControl 0.21.1 · One Windows download

> **Historical release notes / Notas históricas.** Instructions below belong to this version. For current installation and use, see [English](../README.md) / [Español](../README.es.md).

SeamlessControl connects Omarchy and Windows in one workspace. Move the pointer across a screen edge to work on another computer; your physical keyboard, tested shortcuts and text clipboard follow you. Return across the edge or with Escape. Nearby computers can be discovered on the private LAN, and pairing requires the same six digit code on both screens. Files are offered with receiver approval and verified before delivery.

This release makes Windows setup simpler: **download one ZIP** instead of collecting two separate executables. The archive contains `seamlesscontrol.exe` (the bilingual tray app) and `seamlesscontrold.exe` (its agent) together. The ZIP is built from this exact tag; its adjacent `.sha256` asset lets you verify the download. The Omarchy plugin and agent behavior from 0.21.0 is unchanged.

## Install or update

**Windows x64:** Under **Assets**, download `seamlesscontrol-windows-x64.zip`, extract it into a folder you own, and open `seamlesscontrol.exe`. To update, exit the app from its tray icon, extract the new ZIP over the two executables, and reopen it. Your pairing data under `%LOCALAPPDATA%\SeamlessControl` stays in place. See the [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.1/docs/WINDOWS-ALPHA.md).

**Omarchy:** Install with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, then choose **Install agent** in the panel. To update, use `omarchy plugin update seamlesscontrol.control`, restart the shell and choose **Update agent**. See the [Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.1/docs/USER-GUIDE.md).

Physical Omarchy↔Windows control, bidirectional text clipboard and an Omarchy→Windows file delivery were confirmed on two computers. The [test record](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.1/docs/TEST-RESULTS.md) describes the observed combinations and additional scenarios. We will keep improving SeamlessControl.

---

# SeamlessControl 0.21.1 · Una sola descarga para Windows

SeamlessControl une Omarchy y Windows: el ratón y teclado físicos cruzan entre pantallas, los atajos probados funcionan en el destino y el portapapeles de texto se sincroniza. Vuelve por el borde o con Escape. El descubrimiento encuentra equipos en la LAN privada; el emparejamiento exige comparar el código de seis cifras en ambas pantallas. Cada archivo enviado requiere la aprobación del receptor.

Ahora basta **un ZIP** para Windows x64. En **Assets**, descarga `seamlesscontrol-windows-x64.zip`, extráelo y abre `seamlesscontrol.exe`; el archivo ya incluye `seamlesscontrold.exe` junto a la app. El `.sha256` contiguo permite verificar la descarga. Para actualizar, sal desde la bandeja, extrae el nuevo ZIP sobre ambos ejecutables y abre la aplicación otra vez. Se conservan los emparejamientos. La [guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.1/docs/WINDOWS-ALPHA.es.md) explica el proceso paso a paso.

En Omarchy, instala con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` y pulsa **Instalar agente**. La [guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.21.1/docs/USER-GUIDE.es.md) cubre también la actualización. El comportamiento del agente Omarchy de 0.21.0 no cambia.
