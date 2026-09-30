# SeamlessControl

Plugin nuevo para Omarchy que busca compartir un teclado y ratón entre equipos. El primer objetivo es Omarchy ↔ Omarchy; después se ampliará a Windows ↔ Omarchy. [Plan completo](./PLAN.md) · [Plan visual](./index.html).

## Estado actual

Se ha validado en un Omarchy 4.0.4 con Hyprland 0.56.2 que el portal `InputCapture` acepta una barrera y entrega movimiento relativo del ratón físico por EIS al cruzarla. Hyprland permite crear dispositivos virtuales de teclado y puntero sin privilegios de sistema. El agente experimental ya une la captura, el canal Noise XX y la inyección, pero **el recorrido entre dos equipos todavía no se ha probado**. No hay una versión lista para uso cotidiano.

## Instalar en cada Omarchy

Use los comandos habituales de plugins de Omarchy y luego instale el agente desde el plugin recién descargado:

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh
```

El instalador compila el agente e instala automáticamente, si faltan, Rust, Avahi y `wl-clipboard` mediante `omarchy pkg add`; activa Avahi para descubrir equipos en la red. Puede pedir la contraseña de `sudo` para paquetes o el servicio. La primera compilación descarga dependencias de Cargo. El binario queda en `~/.local/bin/seamlesscontrold` y las unidades de usuario se copian sin activarlas. Si la sesión gráfica no incluye `~/.local/bin` en `PATH`, añádalo y vuelva a iniciar la sesión para que el panel encuentre el agente.

Para desinstalar, termine primero la sesión de control desde el panel o el terminal y ejecute, en este orden:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/uninstall-agent.sh
omarchy plugin remove seamlesscontrol.control
```

La retirada conserva la identidad, los pares y la cuadrícula en `~/.config/seamlesscontrol/`; puede reinstalar sin volver a emparejar. Si hay un agente activo fuera de las unidades de usuario, el script pide cerrarlo antes de quitar el binario.

## Probar el agente experimental entre dos Omarchy

Instale la misma revisión en ambos equipos con los comandos anteriores. En el destino, pulse **Recibir control** dejando vacía la dirección: el agente elige su IP LAN y anuncia el receptor mediante Avahi/mDNS. En el origen, el panel muestra **Equipos en la red**; pulse **Emparejar**, compare el código en ambos paneles y apruebe. Ubique el destino junto al origen en la cuadrícula y pulse **Compartir** en su fila. La búsqueda sólo propone una dirección; no reemplaza la aprobación ni el cifrado. El panel puede terminar los procesos que haya iniciado.

Si cambia la IP de un equipo emparejado y conserva su identidad, **Recibir control** detecta la dirección nueva, vuelve a escuchar y actualiza el anuncio mDNS automáticamente. El panel del origen mostrará **Actualizar IP**. Una conexión Noise comprueba la clave ya fijada antes de trasladar el par y su posición en la cuadrícula; no se repite el código. Si aparece **Clave cambió** en una IP ya ocupada, el panel impide la conexión hasta que se resuelva el conflicto. El anuncio mDNS por sí solo nunca modifica un par guardado.

La detección automática necesita multicast mDNS disponible en la LAN. Si la red lo bloquea, el panel conserva los campos de IP manual. También puede usar terminales o servicios de usuario. Para iniciar un receptor sin escribir su IP:

Con varias interfaces, el agente intenta elegir la IP de la ruta multicast mDNS. Puede comprobar la dirección elegida con `seamlesscontrold local-address 47832`; si la red no permite esta detección, use la alternativa manual.

```bash
seamlesscontrold serve-auto 47832
```

En el origen, `seamlesscontrold discover` muestra nombre, IP, puerto y huella anunciados. Use el panel para emparejar sin copiar la IP. La huella del anuncio mDNS es orientativa: compare el código de seis cifras que aparece durante el emparejamiento en ambos equipos. La alternativa manual en el destino es:

```bash
seamlesscontrold serve 192.168.1.20:47832
```

Sustituya `192.168.1.20` por la IP LAN de ese equipo. En el equipo que tiene teclado y ratón físicos, ejecute:

```bash
seamlesscontrold connect 192.168.1.20:47832 right
```

También puede emparejar antes de iniciar la captura con `seamlesscontrold pair 192.168.1.20:47832`, o introducir esa dirección en el campo manual del panel del equipo de origen.

Después del emparejamiento, coloque ambos equipos en la cuadrícula 2×2 del panel. La misma operación está disponible por terminal:

```bash
seamlesscontrold topology set 192.168.1.20 1 0
seamlesscontrold topology
seamlesscontrold connect 192.168.1.20:47832
```

La posición local inicial es `(0,0)`. En este ejemplo, el borde de salida se deduce como `right`. `topology set local <columna> <fila>` mueve este equipo; si ambas casillas están ocupadas, intercambia sus posiciones. `topology remove <IP>` quita un par del mapa sin revocar su clave. Las posiciones se guardan en `~/.config/seamlesscontrol/topology`. `topology route <IP>` muestra los saltos por casillas contiguas y comprueba que cada salto tiene un par fijado. Una conexión `connect` sin borde explícito requiere que el par esté en una casilla contigua, nunca diagonal. Si se cambia la cuadrícula durante `mesh` o durante ese `connect` deducido, el agente libera la captura activa y la reconstruye con la disposición nueva; `connect` con un borde explícito conserva el borde elegido.

### Malla experimental de hasta cuatro Omarchy

Empareje previamente **desde el equipo con teclado y ratón físicos** cada destino y guarde las mismas posiciones de la cuadrícula 2×2 en todos los equipos (cada uno con su propia casilla `local` y las IP de los otros). Todos deben usar esta misma revisión del agente y escuchar en el mismo puerto. En cada destino, inicie `seamlesscontrold serve <IP-del-destino>:47832`; en el origen, ejecute:

```bash
seamlesscontrold mesh 47832
```

El origen instala barreras sólo en los bordes que llevan a una casilla vecina. Al entrar en un destino, el agente sitúa el cursor unos píxeles dentro del borde correspondiente y conserva la altura o anchura relativa del cruce entre monitores de distinta resolución. Cuando el cursor de un destino alcanza el borde hacia otro destino, el receptor envía `SWITCH` con la posición; el origen comprueba la vecindad y la época, establece la conexión cifrada al siguiente, pide `RELEASE` al anterior y espera `ENDED` antes de enviar `BEGIN` al nuevo. Las teclas y botones que siguen pulsados se reproducen en el nuevo destino; el movimiento durante esa breve cesión se descarta. `Escape` o el botón **Devolver control al origen** recuperan el puntero. Tras un fallo de transporte, el origen libera el portal, cierra las conexiones, espera de 1 a 30 segundos y vuelve a preparar la captura; un nuevo cruce establece otra sesión cifrada. Esta ruta de malla está implementada y pasa pruebas de política y protocolo, pero **no se ha probado con varios equipos físicos**. También puede iniciarse desde **Compartir en malla** en el panel Omarchy. El origen distribuye el portapapeles de texto UTF-8 entre los destinos conectados; cuando uno se conecta después, recibe el último cambio conocido. No se han probado físicamente las copias simultáneas entre tres o cuatro equipos.

Para que el destino solicite automáticamente la vuelta al cruzar el borde hacia el origen, configure también su mapa en sentido inverso. Si el origen es `192.168.1.10` y queda a la izquierda del destino, ejecute **en el destino**:

```bash
seamlesscontrold topology set local 1 0
seamlesscontrold topology set 192.168.1.10 0 0
```

Durante una captura, el receptor consulta la posición global del cursor a través del [IPC de Hyprland](https://wiki.hypr.land/IPC/). Sólo solicita el retorno después de observar que el puntero se alejó del borde y volvió a él. En el retorno automático envía la posición del cruce para que el origen recupere el cursor a una altura o anchura proporcional. Si el mapa no sitúa al origen como vecino, falta el socket de Hyprland o el monitor tiene una rotación aún no validada, sigue disponible el botón manual del panel.

`seamlesscontrold diagnose` consulta ese IPC sin modificar el escritorio y muestra la posición del cursor, los rectángulos lógicos de los monitores y `LOCK unlocked|locked|undetermined`. Ejecútelo dentro de la sesión gráfica para comprobar que el agente puede leerlos. El receptor sólo acepta entrada cuando Hyprland confirma `unlocked`; si aparece un bloqueo o falla esa lectura, corta el canal y libera teclas y botones retenidos. Si se bloquea el origen, éste suspende la captura y el portapapeles hasta verificar el desbloqueo. Este comportamiento todavía requiere una prueba física de bloqueo durante el control remoto.

Con un par ya emparejado y el receptor `serve` en marcha, `seamlesscontrold latency 192.168.1.20:47832` intercambia veinte solicitudes y respuestas por el mismo canal cifrado. La línea `LATENCY` informa el mínimo, p50, p95 y máximo en microsegundos, en ese orden. La medición excluye el tiempo de emparejamiento y no representa el retraso de captura, inyección ni visualización; esos tiempos requieren dos equipos y entrada física. Este comando no crea un par nuevo.

`right` puede cambiarse por `left`, `top` o `bottom` según el borde de salida. La primera conexión muestra un código de seis cifras en ambos paneles Omarchy y en ambos terminales. **Compare los códigos en los dos equipos y apruebe en ambos paneles sólo si coinciden.** Como alternativa por terminal, consulte `seamlesscontrold status` y ejecute `seamlesscontrold approve 123456` con el código mostrado; `seamlesscontrold reject` deniega el par. Las claves y sus IP actuales se guardan en `~/.config/seamlesscontrol/peers/`, con permisos privados. La captura puede solicitar consentimiento del portal. Cruce el borde elegido para enviar teclado y ratón al destino; pulse **Escape** para devolver el control local, o use **Devolver control al origen** en el panel del destino (`seamlesscontrold return` por terminal). **Ctrl+C** en el terminal de origen cierra el agente. `seamlesscontrold pause` y `resume`, o el botón del panel, desactivan y reactivan la captura. La conexión termina si fallan los latidos; el receptor libera teclas y botones que hayan quedado pulsados.

El portapapeles de **texto UTF-8** se sincroniza durante `connect` y `mesh`, incluso cuando el puntero permanece en el origen. El instalador incluye `wl-copy` y `wl-paste` mediante `wl-clipboard`. Cada cambio viaja por un canal cifrado, con límite de 256 KiB; se ignoran contenidos mayores, otros formatos y selecciones que Wayland marque como sensibles. En `connect`, una revisión y la identidad del par resuelven las copias simultáneas. En `mesh`, el origen autentica cada emisor y publica una revisión nueva para los demás destinos; las copias simultáneas en varios equipos requieren validación física. Instale esta misma revisión del agente en todos los extremos: el saludo de aplicación `seamlesscontrol/5` rechaza agentes anteriores.

## Enviar un archivo con confirmación

Después de emparejar los equipos, puede usar la sección **Archivos** del panel Omarchy. En el destino deje vacía la dirección, elija el directorio y pulse **Esperar un archivo**: el agente elegirá su IP LAN y el puerto 47833. En el origen pulse **Enviar a** junto al equipo emparejado, elija el archivo y pulse **Enviar archivo**. El destino verá el equipo, nombre, tamaño y SHA-256 y debe pulsar **Aceptar archivo** o **Rechazar**. El panel conserva el estilo del resto del plugin y utiliza el mismo protocolo cifrado que la CLI. Si prefiere terminal, abra **en el destino** otro terminal y el mismo puerto LAN:

```bash
seamlesscontrold receive-file 192.168.1.20:47833 ~/Downloads
```

La variante `seamlesscontrold receive-file-auto 47833 ~/Downloads` elige la IP local sin escribirla. El origen aún necesita la dirección del destino si usa la CLI; `seamlesscontrold discover` la muestra mientras su receptor de control esté activo.

En el origen ejecute `seamlesscontrold send-file 192.168.1.20:47833 /ruta/al/archivo`. El destino muestra nombre, tamaño y SHA-256 y sólo escribe el archivo si alguien responde exactamente `SI` en ese terminal. Es una transferencia por comando: cada ejecución de `receive-file` acepta una sola conexión. Puede ejecutarse junto al agente de control si utiliza **otro puerto**. El archivo llega por un canal Noise autenticado con las claves ya emparejadas; el destino lo guarda temporalmente y verifica tamaño y SHA-256 antes de publicarlo. Si el origen crece durante el envío, la sesión se cancela y el destino elimina el temporal. Si existe un archivo con el mismo nombre, la transferencia falla sin reemplazarlo. El límite predeterminado es 100 MiB en ambos equipos; `SEAMLESSCONTROL_MAX_FILE_BYTES` permite cambiarlo en cada comando o en el entorno de `omarchy-shell`. Ambos agentes deben tener la misma versión del protocolo de archivos.

Esta prueba requiere dos sesiones Omarchy reales en la misma LAN. Hoy sólo está disponible una; la lista de pruebas físicas está en [docs/TESTING.md](./docs/TESTING.md). El emisor `connect` reintenta las pérdidas de red y un receptor ocupado con esperas de 1 a 30 segundos y reutiliza el par fijado; su recuperación real tras una caída de Wi-Fi aún requiere pruebas físicas. El receptor puede atender diagnósticos cifrados de otros pares mientras una sesión conserva la reserva exclusiva de entrada. La captura instala barreras en los tramos exteriores del borde elegido para todos los monitores anunciados y las recalcula cuando el portal notifica un cambio de zonas; falta validar ese flujo con monitores reales. El retorno manual y el detector de borde remoto usan el canal cifrado; **el retorno automático, la malla, el portapapeles y los archivos aún no se han probado físicamente**. Windows sigue pendiente.

## Widget de Omarchy

El repositorio incluye un `manifest.json` válido y un widget que muestra el estado real del agente, descubre receptores LAN, permite iniciar y terminar una sesión, emparejar, aprobar, pausar la captura, devolver el control desde el destino, organizar la cuadrícula y revocar equipos. Instálelo con los comandos del inicio. Si falta el ejecutable, el panel indica cómo instalarlo y lo detecta automáticamente después. Tras actualizar el código QML, `omarchy restart shell` fuerza la carga de la revisión nueva si la recarga en caliente conserva un componente anterior. El borde se deduce de la casilla vecina; una sesión iniciada por terminal o servicio de usuario se detiene desde ese mismo medio. Por CLI, `seamlesscontrold peers` enumera los equipos y `seamlesscontrold revoke <IP>` impide que la clave revocada vuelva a conectarse y quita su posición. Con el agente detenido, `seamlesscontrold rotate-key` reemplaza la identidad local; cada equipo remoto deberá revocar la clave anterior y emparejar de nuevo.

Los comandos de desinstalación están al inicio de esta guía. El script deshabilita y detiene las cuatro unidades de usuario instaladas, elimina sus archivos y el binario, y conserva `~/.config/seamlesscontrol/` con identidad, pares y cuadrícula.

El contrato del socket local y los comandos del panel están descritos en [docs/IPC.md](./docs/IPC.md).

## Servicios de usuario

El instalador copia cuatro unidades de `systemd --user` y las deja deshabilitadas. `seamlesscontrol-receiver-auto.service` usa el puerto 47832 y elige la IP LAN sin archivo de entorno. Para la alternativa manual, cree `~/.config/seamlesscontrol/receiver.env` con `SEAMLESSCONTROL_LISTEN=192.168.1.20:47832`. Para un solo destino, cree `~/.config/seamlesscontrol/sender.env` en el origen con `SEAMLESSCONTROL_PEER=192.168.1.20:47832` y `SEAMLESSCONTROL_EDGE=right`. Para varios destinos ya emparejados y situados en la cuadrícula, cree `~/.config/seamlesscontrol/mesh.env` en el origen con `SEAMLESSCONTROL_PORT=47832`. Active **una** unidad por equipo con `systemctl --user enable --now seamlesscontrol-receiver-auto.service`, `seamlesscontrol-receiver.service`, `seamlesscontrol-sender.service` o `seamlesscontrol-mesh.service`, según corresponda. El origen se reinicia tras una caída de red y reutiliza las claves fijadas. Consulte su estado con `systemctl --user status ...` y sus registros con `journalctl --user -u ...`.

Para actualizar en ambos equipos, ejecute `omarchy plugin update seamlesscontrol.control` y después `bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh`. El instalador reemplaza el ejecutable de forma atómica y conserva `~/.config/seamlesscontrol/`. Un agente que ya estaba en ejecución sigue usando la revisión anterior hasta que se detenga y vuelva a iniciar desde el panel, terminal o servicio de usuario; haga ese reinicio en ambos extremos antes de usar un protocolo nuevo.

## Compilar y probar

Con Rust estable y Cargo instalados:

```bash
cargo test --manifest-path agent/Cargo.toml
cargo clippy --manifest-path agent/Cargo.toml --all-targets -- -D warnings
CARGO=cargo bash tests/pair_loopback.sh
CARGO=cargo bash tests/reconnect_wait.sh
CARGO=cargo bash tests/file_loopback.sh
CARGO=cargo bash tests/discovery_lan.sh
CARGO=cargo bash tests/roaming_loopback.sh
CARGO=cargo bash tests/update_installed_agent.sh
CARGO=cargo bash tests/uninstall_agent.sh
cargo build --release --manifest-path agent/Cargo.toml --bin seamlesscontrold
SC_IDLE_SECONDS=30 bash tests/idle_receiver.sh
```

La prueba criptográfica usa sockets en `127.0.0.1`; el entorno de pruebas debe permitirlo. El medidor de reposo inicia un receptor con identidad y directorios temporales, informa ticks de CPU, RSS y descriptores, y lo detiene al terminar; requiere un binario release compilado. No incluye el consumo del daemon Avahi compartido. En un Omarchy con sesión gráfica se pueden ejecutar los probes:

```bash
cargo run --manifest-path agent/Cargo.toml --bin virtual-input-probe
cargo run --manifest-path agent/Cargo.toml --bin capture-probe
cargo run --manifest-path agent/Cargo.toml --bin capture-probe -- --listen
cargo run --manifest-path agent/Cargo.toml --bin capture-probe -- --listen --verify-input
```

El primer probe crea y cierra dispositivos virtuales sin enviar eventos. El segundo solicita permiso al portal, instala temporalmente una barrera en el borde derecho del primer monitor y comprueba la conexión EIS. Con `--listen`, espera hasta 20 segundos por una activación, lee un evento durante un máximo de cinco segundos y libera la captura. `--verify-input` espera hasta ocho segundos tras la activación para confirmar que llegaron teclado y botón. Imprime sólo tipos de eventos, sin registrar la tecla ni el clic. Consulte [los resultados locales](./docs/FEASIBILITY.md).

## Licencia

MIT. Consulte [LICENSE](./LICENSE).
