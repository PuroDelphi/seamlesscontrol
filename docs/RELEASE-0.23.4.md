# SeamlessControl 0.23.4 · File consent follows the computer

This update closes the marketplace review finding about temporary file approvals. When you approve a file for a limited time, that permission now belongs to the sender's authenticated identity. Revoking a pairing clears its temporary permission. If another computer later receives the same IP address, its first file asks for approval again. This applies to both copied files and manual transfers in Omarchy and Windows.

**Ask every time** and **Accept automatically** keep their existing behavior. Mouse, keyboard, text clipboard, pairing, and the encrypted transfer protocol are unchanged. A local encrypted regression test covered revocation, re-pairing a different identity at the same IP, and rejection of the new sender's first offer.

## Update

**Omarchy:** end active sessions, run `omarchy plugin update seamlesscontrol.control --yes`, restart the panel with `omarchy restart shell`, then select **Update agent**. Update both Omarchy computers so the plugin and agent understand the new internal offer format.

**Windows x64:** exit the app from the tray. Under **Assets**, download `seamlesscontrol-windows-x64.zip` and its adjacent `.sha256`, extract the ZIP, replace **both** `seamlesscontrol.exe` and `seamlesscontrold.exe`, then reopen the app. Saved pairings and preferences remain in place.

[Omarchy guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.4/docs/USER-GUIDE.md) · [Windows guide](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.4/docs/WINDOWS.md)

---

# SeamlessControl 0.23.4 · El permiso de archivos pertenece al equipo

Esta actualización corrige el hallazgo de la revisión del marketplace sobre la aprobación temporal de archivos. Cuando apruebas archivos por un tiempo limitado, el permiso queda asociado a la identidad autenticada del remitente. Revocar un emparejamiento borra ese permiso. Si después otro equipo recibe la misma IP, su primer archivo vuelve a pedir aprobación. Esto se aplica a archivos copiados y a transferencias manuales en Omarchy y Windows.

**Preguntar siempre** y **Aceptar automáticamente** conservan su funcionamiento. El ratón, teclado, portapapeles de texto, emparejamiento y protocolo cifrado de transferencia no cambian. Una prueba cifrada local cubrió la revocación, el nuevo emparejamiento de otra identidad con la misma IP y el rechazo de su primera oferta.

## Actualizar

**Omarchy:** termina las sesiones activas, ejecuta `omarchy plugin update seamlesscontrol.control --yes`, reinicia el panel con `omarchy restart shell` y pulsa **Actualizar agente**. Actualiza ambos Omarchy para que el plugin y el agente entiendan el nuevo formato interno de las ofertas.

**Windows x64:** sal de la app desde la bandeja. En **Assets**, descarga `seamlesscontrol-windows-x64.zip` y su `.sha256` contiguo, extrae el ZIP, reemplaza **ambos** ejecutables y abre de nuevo la app. Se conservan los emparejamientos y preferencias.

[Guía Omarchy](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.4/docs/USER-GUIDE.es.md) · [Guía Windows](https://github.com/PuroDelphi/seamlesscontrol/blob/v0.23.4/docs/WINDOWS.es.md)
