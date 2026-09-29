# SeamlessControl

Plugin nuevo para Omarchy que busca compartir un teclado y ratón entre equipos. El primer objetivo es Omarchy ↔ Omarchy; después se ampliará a Windows ↔ Omarchy. [Plan completo](./PLAN.md) · [Plan visual](./index.html).

## Estado actual

Se ha validado en un Omarchy 4.0.4 con Hyprland 0.56.2 que el portal `InputCapture` acepta una barrera y entrega una conexión EIS, y que Hyprland permite crear dispositivos virtuales de teclado y puntero sin privilegios de sistema. El agente experimental ya une la captura, el canal Noise XX y la inyección, pero **el recorrido entre dos equipos todavía no se ha probado**. No hay una versión lista para uso cotidiano.

## Probar el agente experimental entre dos Omarchy

En ambos equipos, ejecute `bash packaging/install-agent.sh` para compilar e instalar el binario en `~/.local/bin` (o `XDG_BIN_HOME`). Este directorio debe estar en `PATH` de la sesión gráfica para que el widget pueda consultar el agente. En el equipo que recibirá el control, ejecute:

```bash
seamlesscontrold serve 192.168.1.20:47832
```

Sustituya `192.168.1.20` por la IP LAN de ese equipo. En el equipo que tiene teclado y ratón físicos, ejecute:

```bash
seamlesscontrold connect 192.168.1.20:47832 right
```

También puede emparejar antes de iniciar la captura con `seamlesscontrold pair 192.168.1.20:47832`, o introducir esa dirección en el panel del widget del equipo de origen.

Después del emparejamiento, coloque ambos equipos en la cuadrícula 2×2 del panel. La misma operación está disponible por terminal:

```bash
seamlesscontrold topology set 192.168.1.20 1 0
seamlesscontrold topology
seamlesscontrold connect 192.168.1.20:47832
```

La posición local inicial es `(0,0)`. En este ejemplo, el borde de salida se deduce como `right`. `topology set local <columna> <fila>` mueve este equipo; si ambas casillas están ocupadas, intercambia sus posiciones. `topology remove <IP>` quita un par del mapa sin revocar su clave. Las posiciones se guardan en `~/.config/seamlesscontrol/topology`. Una conexión sin borde explícito requiere que el par esté en una casilla contigua, nunca diagonal.

Para que el destino solicite automáticamente la vuelta al cruzar el borde hacia el origen, configure también su mapa en sentido inverso. Si el origen es `192.168.1.10` y queda a la izquierda del destino, ejecute **en el destino**:

```bash
seamlesscontrold topology set local 1 0
seamlesscontrold topology set 192.168.1.10 0 0
```

Durante una captura, el receptor consulta la posición global del cursor a través del [IPC de Hyprland](https://wiki.hypr.land/IPC/). Sólo solicita el retorno después de observar que el puntero se alejó del borde y volvió a él. Si el mapa no sitúa al origen como vecino, falta el socket de Hyprland o el monitor tiene una rotación aún no validada, sigue disponible el botón manual del panel.

`seamlesscontrold diagnose` consulta ese IPC sin modificar el escritorio y muestra la posición del cursor y los rectángulos lógicos de los monitores. Ejecútelo dentro de la sesión gráfica para comprobar que el agente puede leerlos.

`right` puede cambiarse por `left`, `top` o `bottom` según el borde de salida. La primera conexión muestra un código de seis cifras en ambos paneles Omarchy y en ambos terminales. **Compare los códigos en los dos equipos y apruebe en ambos paneles sólo si coinciden.** Como alternativa por terminal, consulte `seamlesscontrold status` y ejecute `seamlesscontrold approve 123456` con el código mostrado; `seamlesscontrold reject` deniega el par. Las claves quedan fijadas por IP en `~/.config/seamlesscontrol/peers/`, con permisos privados. La captura puede solicitar consentimiento del portal. Cruce el borde elegido para enviar teclado y ratón al destino; pulse **Escape** para devolver el control local, o use **Devolver control al origen** en el panel del destino (`seamlesscontrold return` por terminal). **Ctrl+C** en el terminal de origen cierra el agente. `seamlesscontrold pause` y `resume`, o el botón del panel, desactivan y reactivan la captura. La conexión termina si fallan los latidos; el receptor libera teclas y botones que hayan quedado pulsados.

El portapapeles de **texto UTF-8** se sincroniza en ambos sentidos mientras la conexión está activa, incluso cuando el puntero permanece en el origen. Requiere `wl-copy` y `wl-paste` en ambos equipos. Cada cambio viaja por el canal cifrado, con límite de 256 KiB; se ignoran contenidos mayores, otros formatos y selecciones que Wayland marque como sensibles. Si ambos equipos copian a la vez, una revisión y la identidad del par resuelven el empate de forma determinista. Instale esta misma revisión del agente en ambos extremos: el saludo de aplicación `seamlesscontrol/2` rechaza agentes anteriores.

## Enviar un archivo con confirmación

Después de emparejar los equipos, abra **en el destino** otro terminal y otro puerto LAN (por ejemplo, `47833`):

```bash
seamlesscontrold receive-file 192.168.1.20:47833 ~/Downloads
```

En el origen ejecute `seamlesscontrold send-file 192.168.1.20:47833 /ruta/al/archivo`. El destino muestra nombre, tamaño y SHA-256 y sólo escribe el archivo si alguien responde exactamente `SI` en ese terminal. Es una transferencia por comando: cada ejecución de `receive-file` acepta una sola conexión. Puede ejecutarse junto al agente de control si utiliza **otro puerto**. El archivo llega por un canal Noise autenticado con las claves ya emparejadas; el destino lo guarda temporalmente y verifica tamaño y SHA-256 antes de publicarlo. Si existe un archivo con el mismo nombre, la transferencia falla sin reemplazarlo. El límite predeterminado es 100 MiB en ambos equipos; `SEAMLESSCONTROL_MAX_FILE_BYTES` permite cambiarlo en cada comando. Ambos agentes deben tener la misma versión del protocolo de archivos. El panel de Omarchy aún no ofrece selección ni aprobación de archivos.

Esta prueba requiere dos sesiones Omarchy reales en la misma LAN. Hoy sólo está disponible una; la lista de pruebas físicas está en [docs/TESTING.md](./docs/TESTING.md). El emisor reintenta las pérdidas de red con esperas de 1 a 30 segundos y reutiliza el par fijado; su recuperación real tras una caída de Wi-Fi aún requiere pruebas físicas. La captura instala barreras en los tramos exteriores del borde elegido para todos los monitores anunciados y las recalcula cuando el portal notifica un cambio de zonas; falta validar ese flujo con monitores reales. La cuadrícula guarda hasta cuatro posiciones, pero el agente controla un solo par por conexión. El retorno manual y el detector de borde remoto usan el canal cifrado; **el retorno automático, el portapapeles y los archivos aún no se han probado físicamente**. El enrutamiento entre cuatro equipos, la integración visual de archivos y Windows siguen pendientes.

## Widget de Omarchy

El repositorio incluye un `manifest.json` válido y un widget que muestra el estado real del agente, permite iniciar el emparejamiento, aprobarlo, pausar la captura, devolver el control desde el destino, organizar la cuadrícula y revocar equipos. Se puede instalar con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol --enable`. **El inicio de la sesión de control todavía se hace en terminal o mediante servicio de usuario**; el borde se puede deducir de la cuadrícula. Por CLI, `seamlesscontrold peers` enumera los equipos y `seamlesscontrold revoke <IP>` impide que la clave revocada vuelva a conectarse y quita su posición. Para desinstalar el widget, use `omarchy plugin remove seamlesscontrol.control`; el binario puede eliminarse de `~/.local/bin` y las claves persistentes quedan en `~/.config/seamlesscontrol/` hasta que el usuario decida borrarlas.

El contrato del socket local y los comandos del panel están descritos en [docs/IPC.md](./docs/IPC.md).

## Servicios de usuario

El instalador copia dos unidades de `systemd --user` y las deja deshabilitadas. En el equipo receptor, cree `~/.config/seamlesscontrol/receiver.env` con `SEAMLESSCONTROL_LISTEN=192.168.1.20:47832`. En el emisor, cree `~/.config/seamlesscontrol/sender.env` con `SEAMLESSCONTROL_PEER=192.168.1.20:47832` y `SEAMLESSCONTROL_EDGE=right`. Tras comprobar los valores, active **una** unidad por equipo con `systemctl --user enable --now seamlesscontrol-receiver.service` o `systemctl --user enable --now seamlesscontrol-sender.service`. El emisor se reinicia tras una caída de red y reutiliza la clave fijada. Consulte su estado con `systemctl --user status ...` y sus registros con `journalctl --user -u ...`.

## Compilar y probar

Con Rust estable y Cargo instalados:

```bash
cargo test --manifest-path agent/Cargo.toml
cargo clippy --manifest-path agent/Cargo.toml --all-targets -- -D warnings
CARGO=cargo bash tests/pair_loopback.sh
CARGO=cargo bash tests/reconnect_wait.sh
```

La prueba criptográfica usa sockets en `127.0.0.1`; el entorno de pruebas debe permitirlo. En un Omarchy con sesión gráfica se pueden ejecutar los probes:

```bash
cargo run --manifest-path agent/Cargo.toml --bin virtual-input-probe
cargo run --manifest-path agent/Cargo.toml --bin capture-probe
cargo run --manifest-path agent/Cargo.toml --bin capture-probe -- --listen
```

El primer probe crea y cierra dispositivos virtuales sin enviar eventos. El segundo solicita permiso al portal, instala temporalmente una barrera en el borde derecho del primer monitor y comprueba la conexión EIS. Con `--listen`, espera hasta 20 segundos por una activación, lee un evento durante un máximo de cinco segundos y libera la captura. Sólo imprime si llegó un evento, sin registrar su contenido. Consulte [los resultados locales](./docs/FEASIBILITY.md).

## Licencia

MIT. Consulte [LICENSE](./LICENSE).
