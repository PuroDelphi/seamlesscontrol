# Agente Windows x64 · alpha

[English](WINDOWS-ALPHA.md) · [Guía de Omarchy](../README.es.md) · [Resultados de pruebas](TEST-RESULTS.es.md)

El agente de consola para Windows usa la misma identidad Noise fijada, el código de emparejamiento, la versión del protocolo y el formato de archivos que Omarchy. Esta versión `alpha` está dirigida a **Windows 10/11 de 64 bits en x86-64**. Permite controlar Windows desde Omarchy y regresar por el borde de entrada. La sincronización de **texto** del portapapeles ya se comprobó físicamente en ambos sentidos. Windows también puede emparejarse y enviar o recibir un archivo aprobado. Windows también puede ser el **origen** del ratón y teclado; se comprobaron físicamente el cruce, clic, tecla, regreso por el borde y regreso con Escape. Quedan para próximos pasos el descubrimiento automático y un panel gráfico para Windows.

## Obtener la compilación x64

Abre la [compilación Windows x64 verificada](https://github.com/PuroDelphi/seamlesscontrol/actions/runs/37098835696) de `alpha`. Al final de la página, en **Artifacts**, descarga `seamlesscontrol-windows-x64-alpha`. Extrae `seamlesscontrold.exe` y su archivo `.sha256` en una carpeta de tu cuenta de Windows. No hay instalador: se ejecuta desde esa carpeta. El workflow lo compila para `x86_64-pc-windows-msvc` con el entorno C integrado; es un artefacto de prueba sin firma. En PowerShell, `(Get-FileHash -Algorithm SHA256 .\seamlesscontrold.exe).Hash.ToLowerInvariant()` debe coincidir con el primer campo del archivo `.sha256`.

También puedes compilar el código de `alpha` en Windows x64 con Rust instalado:

```powershell
cargo build --locked --manifest-path agent/Cargo.toml --target x86_64-pc-windows-msvc --release --bin seamlesscontrold
```

### Actualizar el agente de Windows

Termina primero la sesión en el panel de Omarchy. En Windows, detén `serve` y `receive-file` con Ctrl+C. Descarga el artefacto de la compilación indicada arriba, comprueba su SHA-256, reemplaza `seamlesscontrold.exe` en la carpeta donde ya lo usas y vuelve a iniciar `serve`. La identidad y los emparejamientos guardados en `%LOCALAPPDATA%\SeamlessControl` se conservan. No hace falta volver a emparejar.

## Conectar Omarchy con Windows

1. Pon ambos equipos en la misma red local privada. En Windows, abre PowerShell en la carpeta donde extrajiste el `.exe` e inicia el receptor: `.\seamlesscontrold.exe serve 0.0.0.0:47832`. Deja la terminal abierta. Si Windows pregunta por el acceso a la red, elige solamente **Redes privadas**. Si el firewall bloquea la recepción, revisa y aplica la regla de abajo en **PowerShell como administrador**.
2. En Windows, ejecuta `ipconfig` en otra terminal y anota la dirección **IPv4** de la red local. En Omarchy, abre SeamlessControl. En **Equipos en la red**, escribe esa dirección con el puerto `47832` en el campo manual `IP:puerto` y pulsa **Emparejar**. Windows aún no se anuncia mediante mDNS.
3. Ambos equipos muestran un código de seis cifras. Compáralos. Escribe ese mismo código en la terminal Windows y apruébalo en el panel de Omarchy. La huella larga de identidad es otro dato; no reemplaza el código de emparejamiento.
4. Ya emparejado, en **Mapa de equipos** de Omarchy coloca Windows junto a **Este equipo** en el lado por donde cruzarás. En **Más**, escribe la dirección Windows `IP:47832` en **Conectar por IP** y pulsa ese botón. Espera a **Listo** y cruza el borde exterior elegido con el ratón físico. Para volver, mueve el puntero al menos 17 píxeles hacia el interior de Windows y cruza de regreso el borde por donde entró; **Escape** en el teclado físico de Omarchy también devuelve el control.

El receptor Windows obtiene el borde de regreso del mensaje autenticado `BEGIN`. Por ahora usa los límites del escritorio virtual como geometría; comienza las pruebas con una disposición sencilla de monitores. Mantén accesible una terminal de Omarchy para detener la conexión si hace falta.

Si Omarchy muestra `PeerKeyChanged` antes de presentar el código, la IP de Windows ya pertenece a otra identidad emparejada; puede pasar cuando Windows y otro Omarchy reciben la misma IP por DHCP en momentos distintos. Es una protección de la asociación anterior. **No revoques ese Omarchy si piensas volver a usarlo**: la revocación bloquea su clave. Para conservar ambas asociaciones en esta versión, necesitan direcciones IP diferentes o un cambio temporal y reversible del perfil de pares; la selección de perfiles desde la interfaz aún está pendiente.

### Reglas opcionales del firewall de Windows

Úsalas solo si el firewall bloqueó la conexión y autorizas estas reglas para la red privada. Permiten TCP entrante desde la subred local en dos puertos separados. El puerto `47833` solo se necesita cuando Windows recibe un archivo.

```powershell
New-NetFirewallRule -DisplayName 'SeamlessControl control (Private LAN)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 47832 -Profile Private -RemoteAddress LocalSubnet
New-NetFirewallRule -DisplayName 'SeamlessControl files (Private LAN)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 47833 -Profile Private -RemoteAddress LocalSubnet
```

Para retirarlas, usa `Remove-NetFirewallRule -DisplayName 'SeamlessControl control (Private LAN)'` y el comando equivalente para `SeamlessControl files (Private LAN)`.

## Controlar Omarchy desde Windows · nueva prueba alpha

1. En el panel de Omarchy, termina cualquier sesión **Connect/Conectar** iniciada desde allí. Pulsa **Recibir control** y espera a **Disponible**. El puerto de control es `47832`; si el panel indica que el firewall lo bloquea, usa **Firewall · solo en el receptor → Preparar regla LAN → Autorizar esta regla** para ese puerto.
2. Actualiza el `.exe` de Windows con la compilación enlazada arriba. En otra PowerShell de la misma carpeta, ejecuta `.\seamlesscontrold.exe connect IP_OMARCHY:47832 left` si Omarchy está físicamente a la izquierda de Windows. Cambia `left` por `right`, `top` o `bottom` según la disposición real. Ya emparejados, no necesitas repetir el código. La terminal debe mostrar `Ready to control`.
3. Cruza con el ratón físico el **borde exterior** de Windows indicado. Comprueba el movimiento y después un clic y una tecla inocua en Omarchy. Para volver a Windows, cruza el borde por donde apareció el puntero en Omarchy o pulsa **Escape** en el teclado físico de Windows. Si se pierde la conexión, Windows deja de capturar la entrada local.

Este modo usa ganchos de teclado y ratón de Windows. El 2026-10-03, una pareja física de Windows 11 x64 y Omarchy confirmó el cruce, clic, una tecla, las combinaciones probadas con Windows/Super, pegado de texto, regreso por el borde y regreso con Escape. Mantén la terminal de Windows abierta durante la prueba; Ctrl+C finaliza `connect`. No inicies a la vez una conexión Omarchy→Windows desde el panel.

## Probar el portapapeles de texto

Con `serve` abierto en Windows y la conexión en **Listo** en Omarchy, copia una frase inocua en Omarchy y pégala en una aplicación de Windows. Luego copia otra frase en Windows y pégala en Omarchy. Espera hasta dos segundos después de cada copia para que el receptor detecte el cambio. La sincronización es de texto; imágenes y otros formatos no se transmiten. El usuario confirmó físicamente ambos sentidos el 2026-10-03.

## Transferir un archivo en cualquier sentido

Empareja primero los dos equipos. Para recibir **un** archivo en Windows, abre otra PowerShell en la carpeta del `.exe` y ejecuta `.\seamlesscontrold.exe receive-file 0.0.0.0:47833 "$env:USERPROFILE\Downloads"`. Deja abierta la terminal de `serve`. En Omarchy, abre **Más → Archivos → Enviar archivos**, elige Windows y un archivo local, y usa el puerto `47833`. Windows muestra la oferta y exige escribir **SI** antes de guardar. Ejecuta `receive-file` otra vez para recibir otro archivo.

Para enviar desde Windows a un receptor Omarchy que ya esté esperando en su puerto de archivos, ejecuta `./seamlesscontrold.exe send-file IP_OMARCHY:47833 C:\ruta\archivo.txt`. Aprueba la oferta en Omarchy. El destino comprueba el tamaño y SHA-256 antes de publicar el archivo.

## Alcance de las pruebas y seguridad

- Esta versión de consola requiere un escritorio Windows con sesión iniciada y desbloqueada. Antes de conceder control y al aplicar entrada nueva, comprueba que el escritorio activo sea el interactivo normal; si el estado es desconocido o está bloqueado, detiene la inyección. Windows puede impedir que `SendInput` controle aplicaciones con mayor nivel de integridad; el agente informa el fallo y libera la entrada que registra tras una desconexión normal.
- Se convierten teclas físicas habituales, modificadores, botones y movimiento relativo. Las teclas sin equivalencia se registran y se omiten. La conversión de la rueda es aproximada y requiere una prueba física.
- El 2026-10-02, un Windows 11 x64 físico se emparejó con Omarchy mediante el campo manual y un código coincidente. La conexión permaneció en **Listo** durante más de 20 segundos. El usuario confirmó cruce Omarchy→Windows, regreso por el borde, clic, una tecla, regreso con Escape y que Super+E abrió el Explorador de Windows. También probó varias combinaciones de teclas adicionales y todas funcionaron en esa sesión. Un archivo de texto enviado desde la UI de Omarchy apareció como oferta, se aceptó y quedó guardado en Windows; Omarchy indicó entrega verificada. La prueba de latencia autenticada completó 20 intercambios. El 2026-10-03 el usuario confirmó que el texto del portapapeles se sincroniza y pega correctamente en ambos sentidos. Quedan pendientes teclas o distribuciones no probadas, rueda, arrastre, archivos en sentido Windows→Omarchy, archivos grandes o interrumpidos, bloqueo, suspensión y otras pantallas.
- Las identidades y claves de pares se guardan en `%LOCALAPPDATA%\SeamlessControl` para el usuario Windows. `seamlesscontrold.exe peers` enumera los equipos confiables y `seamlesscontrold.exe revoke IP_DEL_PAR` revoca uno.

Referencias de las API de Windows: [SendInput y UIPI](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput), [teclas por código físico](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-keybdinput), [detección del escritorio de entrada](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openinputdesktop), [coordenadas del escritorio virtual](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics) y [DPI por monitor](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocessdpiawarenesscontext).
