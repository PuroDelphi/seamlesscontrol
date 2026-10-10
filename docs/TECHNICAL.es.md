# Guía técnica de SeamlessControl

[Guía de usuario](../README.es.md) · [English](TECHNICAL.md) · [Resultados de pruebas físicas](TEST-RESULTS.es.md) · [Registro detallado](FEASIBILITY.md) · [Plan de pruebas](TESTING.md) · [Protocolo local](IPC.md)

La [guía ilustrada del panel](USER-GUIDE.es.md) cubre la conexión y recuperación habituales desde la interfaz.

La [guía Windows](WINDOWS.es.md) explica el uso de la aplicación de bandeja. En Windows x64, `seamlesscontrol.exe` es una interfaz Wry/WebView2 y Tao sobre `seamlesscontrold.exe`; ambos ejecutables deben quedar juntos. La app inicia el control como proceso hijo oculto: en el primer inicio recibe, y después restaura el último modo guardado (recibir, conectar o detenido). Cerrar la ventana la deja en la bandeja; salir desde la bandeja termina sus procesos.

El receptor anuncia `_seamlesscontrol._tcp.local.` con la versión del protocolo y la huella pública mediante `mdns-sd`, mientras la app busca ese servicio. El descubrimiento solo proporciona una dirección: siguen siendo obligatorios la fijación de la clave Noise XX y la aprobación del código de seis cifras.

Los botones de firewall elevan el agente para agregar reglas TCP entrantes de control, envío manual y archivos copiados, y una regla UDP `5353` para mDNS. Se limitan al perfil Privado y LocalSubnet. La interfaz informa que solicitó autorización; el comando elevado informa si la regla se aplicó.

El CI Windows compila la GUI con entorno C estático. El workflow del release compila el commit etiquetado y adjunta un ZIP con ambos ejecutables y un SHA-256 del ZIP como asset contiguo. El hijo oculto usa una ventana invisible como propietario del portapapeles, sin depender de una consola visible.

El receptor de control Windows admite un máximo de ocho conexiones TCP simultáneas y cierra las sobrantes antes de crear otro hilo. El intercambio inicial previo a la autenticación tiene un tiempo de espera de E/S de diez segundos; el plazo mayor para confirmar el código empieza solo cuando termina el intercambio Noise.

Esta guía reúne la operación manual, los paquetes, la seguridad y los límites actuales. Las direcciones siguientes son **ejemplos ficticios**: origen `192.168.50.10`, receptor `192.168.50.20`, LAN `192.168.50.0/24`, interfaz `wlan0`.

## Paquetes y actualizaciones

`omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` instala el widget. Omarchy no ejecuta hooks de instalación del repositorio. El botón **Instalar agente** abre una terminal de Omarchy y ejecuta `packaging/install-agent.sh`. Instala los paquetes ausentes `rust`, `avahi` y `wl-clipboard` con `omarchy pkg add`, compila el binario release y lo instala en `~/.local/bin`. Registra solo los paquetes que instaló en `~/.local/state/seamlesscontrol/installed-packages`. Avahi se activa cuando hace falta para el descubrimiento mDNS. Las cuatro unidades de usuario se instalan, pero quedan deshabilitadas. Los archivos de servicio existentes se conservan si difieren de los incluidos en el plugin.

Alternativa desde terminal:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh
```

Para actualizar el widget en **cada** equipo, ejecuta `omarchy plugin update seamlesscontrol.control --yes` y después `omarchy restart shell` para cargar el QML nuevo. Si cambió el agente, detén cualquier sesión activa, pulsa **Actualizar agente** en el panel (o usa el comando de terminal) y reinicia el agente en ambos extremos. Un proceso en marcha conserva la revisión anterior hasta reiniciarlo. Las claves y el mapa se conservan.

**Retirar agente** ejecuta `uninstall-agent.sh --remove-deps` en una terminal de Omarchy. Detiene y retira los archivos de servicio del plugin sin modificaciones, conserva los modificados, elimina el binario y retira solo los paquetes registrados como instalados por este plugin. Pacman rechazará la retirada de paquetes requeridos por otros. Los paquetes que ya estaban presentes no se registran. La desinstalación CLI predeterminada, sin `--remove-deps`, conserva los paquetes. Ninguna ruta borra `~/.config/seamlesscontrol/` (identidad, pares, mapa). Retira una regla de firewall por separado antes de `omarchy plugin remove seamlesscontrol.control`.

## Conexión manual

En el **receptor**, ejecuta estos comandos en terminales separadas: la escucha se mantiene en primer plano.

```bash
seamlesscontrold serve-auto 47832
```

```bash
seamlesscontrold local-address 47832
```

En el **origen**, descubre el receptor y empareja usando su dirección real:

```bash
seamlesscontrold discover
seamlesscontrold pair 192.168.50.20:47832
```

`serve-auto` selecciona una IPv4 según la ruta mDNS y anuncia `_seamlesscontrol._tcp` mediante Avahi. La búsqueda necesita multicast mDNS; si está bloqueado, puedes conservar `serve-auto` y escribir su dirección manualmente, o usar `serve 192.168.50.20:47832` para una dirección fija. No lances ambos receptores a la vez.

El emparejamiento permanece abierto hasta recibir las dos aprobaciones. Compara el código de seis cifras **en ambos equipos**, desde el panel o con `seamlesscontrold status` en otra terminal de cada equipo. Aprueba allí con `seamlesscontrold approve <código-de-seis-cifras>`; `reject` cancela. La huella anunciada por mDNS es orientativa; la identidad local de 64 caracteres es una huella permanente, no el código de comparación.

Para una conexión directa, basta con ubicar al receptor en el mapa del origen. El receptor aprende el borde de regreso al comenzar el control; no necesita un mapa inverso. En el origen, si el receptor está a la derecha:

```bash
seamlesscontrold topology set 192.168.50.20 1 0
seamlesscontrold connect 192.168.50.20:47832
```

`connect` deduce el borde de las casillas vecinas; un argumento explícito `right`, `left`, `top` o `bottom` lo sustituye. Vuelve cruzando el borde del receptor hacia el origen, pulsando Escape en el teclado físico o ejecutando `seamlesscontrold return` en el receptor. `seamlesscontrold emergency-stop` desconecta y pausa la entrada nueva hasta `seamlesscontrold resume`. En el origen, `pause` y `resume` alternan la captura. Detén un agente iniciado en terminal con Ctrl+C.

Otros comandos: `seamlesscontrold peers`, `topology`, `topology route <IP>`, `revoke <IP>`, `diagnose` y `latency <IP:puerto>`. `diagnose` muestra cursor, monitores y `LOCK unlocked|locked|undetermined`; ejecútalo dentro de la sesión gráfica. `latency` mide mínimo/p50/p95/máximo del viaje cifrado, sin incluir captura ni visualización. `rotate-key` requiere detener el agente y obliga a emparejar de nuevo.

## Firewall

El puerto TCP del receptor debe ser accesible **solo desde la LAN**. El panel muestra interfaz, subred, IP receptora y puerto detectados antes de pedir autorización del sistema. El script acepta `show`, `allow` y `remove` con un puerto:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh show 47832
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh allow 47832
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh remove 47832
```

`allow` y `remove` piden confirmación y usan Polkit en sesión gráfica o sudo en terminal. El panel ya mostró la regla y confirmó el clic, así que pasa `--yes`; sigue siendo necesaria la autorización del sistema. El script no activa UFW. Si cambia la IP o subred del receptor, revisa `sudo ufw status numbered` y elimina manualmente reglas antiguas.

| Uso | Puerto predeterminado en el receptor | Activación |
|---|---|---|
| Control y portapapeles de texto | TCP `47832` | Recibir control |
| Envío manual a una carpeta | TCP `47833` | Esperar un archivo; una oferta por ejecución |
| Copiar y pegar archivos | TCP `47834` | App Windows o widget Omarchy activo |
| Descubrimiento mDNS | UDP `5353` | Receptor anunciado en la LAN |

Cada puerto TCP necesita su propia regla si está bloqueado. La aprobación de archivos no abre puertos ni inicia la recepción manual.

## Protocolo y otras funciones

El origen captura entrada mediante el portal del escritorio y EIS. El receptor la inyecta a través de la entrada virtual de Hyprland. Su teclado virtual usa el mapa XKB del compositor para actualizar los modificadores de Wayland con cada tecla, de modo que combinaciones como Super+V puedan llegar a Hyprland como atajos. Las sesiones de red usan Noise XX con claves de pares fijadas. El receptor permite un solo dueño de entrada y libera teclas y botones retenidos al desconectar. El bloqueo se comprueba mediante el IPC de Hyprland; un estado desconocido o bloqueado impide la inyección. Tras Escape o retorno remoto, el puntero debe alejarse 96 píxeles antes de rearmar el borde y evitar una recaptura inmediata.

Cuando el origen se bloquea durante el control remoto, desactiva la captura del portal antes de enviar la señal por la red. Después espera al desbloqueo antes de crear otra sesión de captura. El borde permanece desarmado hasta que el puntero se aparte. Este comportamiento también se aplica a la captura en malla.
Si las zonas de monitor desaparecen temporalmente durante el bloqueo o el apagado de pantalla, el agente aplaza la instalación de barreras hasta recuperar una geometría válida, en vez de salir al recibir una región de tamaño cero.

Mientras el origen controla activamente otro equipo, el widget de la barra Omarchy asocia un inhibidor de inactividad Wayland a su barra. Así se evita que el bloqueo automático por inactividad de Omarchy interrumpa el trabajo remoto; el usuario aún puede bloquear manualmente. El inhibidor desaparece al devolver el control o descargar el widget.

Para diagnosticar temporalmente el teclado, inicia ambos procesos del agente con `SEAMLESSCONTROL_INPUT_TRACE=1`. El origen informa los tipos de evento EIS y si la captura está activa; el receptor Omarchy informa los eventos de tecla inyectados y cuántas teclas siguen retenidas. No registra códigos de tecla ni texto escrito y está desactivado por defecto. Desactívalo al terminar la prueba.

El portapapeles de texto usa UTF-8, tiene límite de 256 KiB y excluye selecciones sensibles de Wayland. La malla experimental 2×2 usa `seamlesscontrold mesh 47832` en el origen después de emparejar y ubicar todos los receptores en todos los mapas. Autentica cada destino y libera al dueño anterior antes de cambiar al siguiente. Revocar una identidad emparejada cierra su conexión de malla aunque ya no controle la entrada; se descartan sus mensajes pendientes y, si tenía el control, se libera la captura. El portapapeles en malla, los cruces rápidos y la recuperación de red siguen pendientes de validación física con varios equipos.

Los archivos usan otra sesión Noise y aprobación del receptor. En el panel, inicia **Esperar un archivo** en el receptor, elige un equipo emparejado y un archivo en el origen, y aprueba la oferta en el receptor. El puerto de archivos (`47833` por defecto) necesita una regla LAN distinta de la del control; el panel permite prepararla y autorizarla. El panel inicia `seamlesscontrold choose-file en|es` o `choose-folder en|es` como proceso separado; esos comandos consultan al portal del escritorio existente para elegir una ruta local y solo imprimen esa ruta como cadena JSON. No abren el archivo ni inician una transferencia. Los equivalentes CLI para transferir son `receive-file-auto 47833 ~/Downloads` y `send-file 192.168.50.20:47833 /ruta/al/archivo`. La interfaz guarda por equipo un límite en MiB (de 1 MiB a 10 GiB; inicial 100 MiB), pasa `SEAMLESSCONTROL_MAX_FILE_BYTES` a los procesos nuevos y lo aplica en emisor y receptor. Hay que reiniciar una espera de archivos que ya esté activa tras cambiarlo. La CLI conserva la variable de entorno. El nombre ofrecido debe ser un único componente de ruta y no puede contener dos puntos; esto evita que las rutas relativas a una unidad Windows o los flujos alternativos sustituyan la carpeta de recepción aprobada. Comprueba tamaño y SHA-256 antes de publicar. Nunca sobrescribe archivos. Una transferencia física entre dos Omarchy pasó el 2026-10-02; consulta los [resultados](TEST-RESULTS.es.md).

La copia de archivos reutiliza la sesión Noise autenticada en TCP `47834`; el receptor manual de una sola transferencia sigue en `47833`. La app Windows y el widget Omarchy mantienen un receptor de archivos copiados mientras están abiertos. Windows lee archivos y carpetas locales desde `CF_HDROP`. Omarchy lee `text/uri-list` y las cargas `copy`/`cut` de `x-special/gnome-copied-files`, `x-special/mate-copied-files` y `x-special/nautilus-clipboard`; el indicador `application/x-kde-cutselection` evita interpretar «Cortar» de KDE como una copia. La vista Recientes de GNOME puede poner `recent://` en la lista URI mientras el formato GNOME incluye el `file://` local real. Cuando hace falta, GIO puede resolver una referencia `recent://`, `starred://`, `search://` o `trash://` al destino local `file://`. Un archivo individual conserva la transferencia anterior. Varios archivos o una carpeta se empaquetan en un solo grupo SCB1 de hasta 256 entradas, con rutas relativas seguras y un hash por archivo. Se rechazan destinos remotos, enlaces simbólicos y rutas relativas inseguras; el límite configurado cubre el grupo entero. El receptor verifica la oferta autenticada, el SHA-256 del paquete, su manifiesto y cada archivo antes de publicar una carpeta para Pegar. Estos formatos describen compatibilidad, no una prueba física de cada explorador. Los [atributos GIO](https://docs.gtk.org/gio/file-attributes.html) documentan `standard::target-uri`; la [referencia de formatos de CopyQ](https://github.com/hluk/CopyQ/blob/master/docs/faq.rst) describe MIME habituales de los exploradores.

Con un único par, el origen ofrece el archivo automáticamente; con varios, se escoge el destino en la interfaz. El receptor muestra un diálogo Windows prioritario o una notificación Omarchy con botones, además de los controles en Archivos. Rechazar impide enviar el contenido. Tras aprobar, el receptor verifica tamaño y SHA-256, publica el archivo o la carpeta del grupo en una carpeta temporal privada y única y, solo entonces, coloca una referencia local `CF_HDROP` o lista URI en el portapapeles del receptor. El usuario pega en el explorador. La carpeta temporal separa nombres repetidos, tiene una cuota mínima de 1 GiB o cuatro veces el límite por archivo y borra sesiones de más de siete días. El receptor necesita otra regla LAN privada para TCP `47834`. El portapapeles de texto ignora selecciones de archivos para no sustituirlas por texto. El envío manual sigue admitiendo un archivo normal por oferta; los grupos copiados usan el protocolo existente de una oferta y requieren agentes compatibles en ambos equipos. Los archivos virtuales de Shell quedan fuera de su alcance. La aprobación y el pegado de un archivo individual funcionaron físicamente en ambos sentidos entre sistemas; los grupos aún requieren comprobación física.

En ambos flujos, la interfaz decide según **Aprobación de archivos entrantes**. **Preguntar siempre** muestra la oferta y espera una acción; **Aceptar automáticamente** aprueba ofertas autenticadas de pares guardados sin diálogo. El modo temporal pregunta por el primer archivo de cada par y conserva su permiso durante 1–1.440 minutos. Los plazos no sobreviven al reinicio; al vencer un plazo activo, el modo vuelve a Preguntar siempre. Las comprobaciones de clave, nombre, tamaño y espacio siguen aplicándose en los tres modos.

Se instalan, pero no se activan, estas unidades: `seamlesscontrol-receiver-auto.service`, `seamlesscontrol-receiver.service`, `seamlesscontrol-sender.service` y `seamlesscontrol-mesh.service`. El receptor automático usa `47832`. Archivos de entorno manuales en `~/.config/seamlesscontrol/` pueden definir `SEAMLESSCONTROL_LISTEN`, `SEAMLESSCONTROL_PEER`, `SEAMLESSCONTROL_EDGE` o `SEAMLESSCONTROL_PORT`. Activa solo la unidad elegida por equipo.

## Verificación y límites

### Bloqueo de conexión observado en dos Omarchy

Durante una conexión iniciada desde el panel el 2026-10-02, el receptor indicó `serve connected` y había una conexión TCP establecida. El origen permaneció varios minutos en `connect connecting` aunque ya había identificado al receptor. El portal de captura no registró una sesión nueva para ese intento. Tras cerrar el proceso atascado del origen y reiniciar allí el servicio de usuario `xdg-desktop-portal-hyprland`, otra conexión creó una sesión del portal, obtuvo las zonas, instaló la barrera del borde, conectó EIS y capturó el puntero. Esto respalda que se bloqueó la preparación de la captura tras autenticar la conexión de red; no se pudo demostrar qué llamada exacta quedó esperando. Hubo un cambio anterior en la disposición de monitores, pero tampoco se ha demostrado que fuera la causa.

El panel distingue ahora la reconexión de red, la preparación de captura tras autenticación y el estado listo para cruzar. Pasados 15 segundos en preparación, ofrece detener el proceso de origen iniciado desde el panel, reiniciar únicamente su servicio de portal Hyprland y volver a pulsar Conectar. Advierte que otras aplicaciones que comparten pantalla en ese origen pueden verse interrumpidas. La parada normal pasa de SIGINT a SIGTERM después de dos segundos si el agente quedó bloqueado antes de iniciar su bucle principal. El receptor sigue escuchando. Esta recuperación se diseñó a partir del incidente observado; todavía falta probar el botón con un bloqueo real.

```bash
bash tests/firewall_lan.sh
bash tests/agent_setup_packages.sh
bash tests/uninstall_agent.sh
cargo test --manifest-path agent/Cargo.toml
cargo clippy --manifest-path agent/Cargo.toml --all-targets -- -D warnings
```

Las pruebas de loopback de emparejamiento, descubrimiento, archivos, reconexión y cambios de IP están en [TESTING.md](TESTING.md). La captura e inyección integradas necesitan una sesión Omarchy real sin otro agente activo. Dos Omarchy físicos confirmaron retorno por el borde, Escape, Super+V y una transferencia de archivo. La [aplicación Windows x64](WINDOWS.es.md) comparte el protocolo. Pruebas físicas con Windows 11 x64 confirmaron control en ambos sentidos, regreso por el borde y Escape, portapapeles de texto en ambos sentidos, los atajos probados, un archivo Omarchy→Windows y el descubrimiento automático de Windows en el panel Omarchy. Las transferencias grandes, la malla de varios equipos, otras disposiciones, la suspensión y la pérdida de red siguen siendo escenarios adicionales de prueba; el [registro](TEST-RESULTS.es.md) precisa su alcance.
