# Agente Windows x64 · alpha

[English](WINDOWS-ALPHA.md) · [Guía de Omarchy](../README.es.md) · [Resultados de pruebas](TEST-RESULTS.es.md)

El agente de consola para Windows usa la misma identidad Noise fijada, el código de emparejamiento, la versión del protocolo y el formato de archivos que Omarchy. Esta versión `alpha` está dirigida a **Windows 10/11 de 64 bits en x86-64**. Por ahora permite controlar Windows desde Omarchy y regresar al cruzar el borde por donde entró el puntero. Windows también puede emparejarse y enviar o recibir un archivo aprobado. Quedan para los siguientes pasos Windows como **origen** del ratón y teclado, el descubrimiento automático, la sincronización del portapapeles y un panel gráfico para Windows.

## Obtener la compilación x64

Abre la última ejecución exitosa del [workflow Windows x64 alpha](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/windows-alpha.yml) en la rama `alpha` y descarga `seamlesscontrol-windows-x64-alpha`. Extrae `seamlesscontrold.exe` en una carpeta de tu cuenta de Windows. El workflow lo compila para `x86_64-pc-windows-msvc`; es un artefacto de prueba, no un instalador firmado.

También puedes compilar el código de `alpha` en Windows x64 con Rust instalado:

```powershell
cargo build --locked --manifest-path agent/Cargo.toml --target x86_64-pc-windows-msvc --release --bin seamlesscontrold
```

## Conectar Omarchy con Windows

1. Pon ambos equipos en la misma red local privada. En PowerShell de Windows, inicia el receptor: `./seamlesscontrold.exe serve 0.0.0.0:47832`. Deja la terminal abierta. Si Windows pregunta por el acceso a la red, elige solamente **Redes privadas**. Si el firewall bloquea la recepción, revisa y aplica la regla de abajo en **PowerShell como administrador**.
2. En Omarchy, abre SeamlessControl. En **Mapa de equipos**, coloca Windows junto a **Este equipo** en el lado por donde cruzarás. En **Equipos en la red**, escribe la dirección `IP:puerto` de Windows con el puerto `47832` y pulsa **Emparejar**. Windows aún no se anuncia mediante mDNS.
3. Ambos equipos muestran un código de seis cifras. Compáralos. Escribe ese mismo código en la terminal Windows y apruébalo en el panel de Omarchy. La huella larga de identidad es otro dato; no reemplaza el código de emparejamiento.
4. En Omarchy, pulsa **Conectar** para la dirección Windows emparejada. Espera a **Listo** y cruza el borde exterior elegido con el ratón físico. Para volver, mueve el puntero al menos 17 píxeles hacia el interior de Windows y cruza de regreso el borde por donde entró; **Escape** en el teclado físico de Omarchy también devuelve el control.

El receptor Windows obtiene el borde de regreso del mensaje autenticado `BEGIN`. Por ahora usa los límites del escritorio virtual como geometría; comienza las pruebas con una disposición sencilla de monitores. Mantén accesible una terminal de Omarchy para detener la conexión si hace falta.

### Reglas opcionales del firewall de Windows

Úsalas solo si el firewall bloqueó la conexión y autorizas estas reglas para la red privada. Permiten TCP entrante desde la subred local en dos puertos separados. El puerto `47833` solo se necesita cuando Windows recibe un archivo.

```powershell
New-NetFirewallRule -DisplayName 'SeamlessControl control (Private LAN)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 47832 -Profile Private -RemoteAddress LocalSubnet
New-NetFirewallRule -DisplayName 'SeamlessControl files (Private LAN)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 47833 -Profile Private -RemoteAddress LocalSubnet
```

Para retirarlas, usa `Remove-NetFirewallRule -DisplayName 'SeamlessControl control (Private LAN)'` y el comando equivalente para `SeamlessControl files (Private LAN)`.

## Transferir un archivo en cualquier sentido

Empareja primero los dos equipos. Para recibir **un** archivo en Windows, crea una carpeta de destino y ejecuta `./seamlesscontrold.exe receive-file 0.0.0.0:47833 C:\Users\TU_USUARIO\Downloads`. En Omarchy, elige la dirección Windows en el envío y el puerto `47833`. Windows muestra la oferta y exige escribir **SI** antes de guardar. Ejecuta `receive-file` otra vez para recibir otro archivo.

Para enviar desde Windows a un receptor Omarchy que ya esté esperando en su puerto de archivos, ejecuta `./seamlesscontrold.exe send-file IP_OMARCHY:47833 C:\ruta\archivo.txt`. Aprueba la oferta en Omarchy. El destino comprueba el tamaño y SHA-256 antes de publicar el archivo.

## Alcance de las pruebas y seguridad

- Esta versión de consola requiere un escritorio Windows con sesión iniciada y desbloqueada. Windows puede impedir que `SendInput` controle aplicaciones con mayor nivel de integridad; el agente informa el fallo y libera la entrada que registra tras una desconexión normal.
- Se convierten teclas físicas habituales, modificadores, botones y movimiento relativo. Las teclas sin equivalencia se registran y se omiten. La conversión de la rueda es aproximada y requiere una prueba física.
- Todavía no se declara ninguna prueba física Windows↔Omarchy. El workflow de `alpha` comprueba la compilación Windows x64 y el núcleo portable; hace falta un equipo Windows real para verificar emparejamiento, cruce, regreso, atajos, archivos, firewall y bloqueo de pantalla.
- Las identidades y claves de pares se guardan en `%LOCALAPPDATA%\SeamlessControl` para el usuario Windows. `seamlesscontrold.exe peers` enumera los equipos confiables y `seamlesscontrold.exe revoke IP_DEL_PAR` revoca uno.

Referencias de las API de Windows: [SendInput y UIPI](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput), [teclas por código físico](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-keybdinput), [coordenadas del escritorio virtual](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics) y [DPI por monitor](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocessdpiawarenesscontext).
