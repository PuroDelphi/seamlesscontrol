# SeamlessControl 0.23.0 · Pairing that stays in one place

> **Historical release notes / Notas históricas.** Instructions below belong to this version. For current installation and use, see [English](../README.md) / [Español](../README.es.md).

Pairing Omarchy and Windows is easier to follow. The **Computers** screen in both apps now walks through four numbered steps: **1 Pair by address**, **2 Nearby computers**, **3 Paired computers**, and **4 Screen layout**. The trusted computers sit directly above the layout, with an arrow and instructions for placing them. Address entry and discovery are alternative ways to start the same secure pairing; compare the six digit code on both screens and approve on each.

On Omarchy, **Stop receiving to pair** appears beside step 1 when this computer is already receiving control. **Stop receiving** is also on Home, including when the receiver was started outside the current panel. This ends the receiver normally and makes outgoing pairing available. **Cut remote input · emergency** serves a different purpose: it cuts and pauses remote input if control needs to be recovered. If the other computer starts pairing, Omarchy can keep receiving and approve its code without stopping.

On Windows, choosing **Pair** temporarily stops this app's active control mode and restores the previous mode when the request ends. The other computer must continue receiving. **Revoke** is available beside saved peers, and an explicit new Pair request after revocation requires fresh matching codes on both computers. Windows and Omarchy can return control by crossing the entry edge; Escape remains available.

The Omarchy/Windows round trip and return across the entry edge were physically confirmed. The user also confirmed the revised Omarchy pairing flow is easier to use. The Windows pause-and-restore pairing transition is covered by the Windows build and remains to be checked on a physical Windows installation. The [test record](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.0/docs/TEST-RESULTS.md) distinguishes these observations.

## Install or update

**Omarchy:** install with `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable`, then choose **Install agent** in the panel. To update, stop active sessions, run `omarchy plugin update seamlesscontrol.control --yes`, run `omarchy restart shell`, and choose **Update agent** in the panel. Update both Omarchy computers before pairing again after revocation. Existing identities and placements remain saved.

**Windows x64:** exit the app from its tray menu. Under **Assets**, download the single `seamlesscontrol-windows-x64.zip` and its adjacent `.sha256`. Extract it and replace **both** `seamlesscontrol.exe` and `seamlesscontrold.exe` in the same folder, then reopen the app. Pairings and preferences remain saved. The release also includes a tagged source archive and its adjacent SHA256.

See the [Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.0/docs/USER-GUIDE.md) and [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.0/docs/WINDOWS.md).

---

# SeamlessControl 0.23.0 · Emparejar sin saltar entre pantallas

Emparejar Omarchy y Windows resulta más fácil de seguir. La pantalla **Equipos** de ambas aplicaciones presenta cuatro pasos numerados: **1 Emparejar por dirección**, **2 Equipos cercanos**, **3 Equipos emparejados** y **4 Mapa de pantallas**. Los equipos de confianza aparecen justo encima del mapa, con una flecha e instrucciones para ubicarlos. La dirección manual y el descubrimiento son dos formas de iniciar el mismo emparejamiento seguro; compara el código de seis cifras en ambas pantallas y apruébalo en cada una.

En Omarchy, **Detener recepción para emparejar** aparece junto al paso 1 si este equipo ya recibe control. **Detener recepción** también está en Inicio, incluso si el receptor se inició fuera del panel actual. Esta acción termina normalmente el receptor y permite iniciar el emparejamiento. **Cortar entrada remota · emergencia** cumple otra función: corta y pausa la entrada remota cuando hay que recuperar el control. Si el otro equipo inicia el emparejamiento, Omarchy puede seguir recibiendo y aprobar su código sin detenerse.

En Windows, **Emparejar** detiene temporalmente el modo de control activo de esta app y restaura el anterior cuando termina la solicitud. El otro equipo debe continuar recibiendo. **Revocar** aparece junto a los pares guardados; una nueva solicitud explícita después de revocar exige códigos nuevos y coincidentes en ambos equipos. Windows y Omarchy pueden devolver el control cruzando el borde de entrada; Escape sigue disponible.

La ida y vuelta entre Omarchy y Windows, incluido el regreso por el borde de entrada, se confirmó físicamente. El usuario también confirmó que el flujo de emparejamiento revisado en Omarchy es más fácil de usar. La transición de pausa y restauración de Windows pasó la compilación Windows y aún requiere verificación en una instalación física. El [registro de pruebas](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.0/docs/TEST-RESULTS.es.md) distingue estos resultados.

## Instalar o actualizar

**Omarchy:** instala con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` y pulsa **Instalar agente** en el panel. Para actualizar, termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, luego `omarchy restart shell` y pulsa **Actualizar agente**. Actualiza ambos Omarchy antes de volver a emparejarlos tras una revocación. Se conservan identidades y posiciones.

**Windows x64:** sal de la app desde la bandeja. En **Assets**, descarga el único `seamlesscontrol-windows-x64.zip` y su `.sha256` contiguo. Extráelo y reemplaza **ambos** ejecutables, `seamlesscontrol.exe` y `seamlesscontrold.exe`, en la misma carpeta; vuelve a abrir la app. Se conservan emparejamientos y preferencias. El release incluye además el código fuente del tag y su SHA256 contiguo.

Consulta la [guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.0/docs/USER-GUIDE.es.md) y la [guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.0/docs/WINDOWS.es.md).
