# Guía técnica de SeamlessControl

[Guía de usuario](../README.es.md) · [English](TECHNICAL.md) · [Resultados de pruebas físicas](TEST-RESULTS.es.md) · [Registro detallado](FEASIBILITY.md) · [Plan de pruebas](TESTING.md) · [Protocolo local](IPC.md)

La [guía ilustrada del panel](USER-GUIDE.es.md) cubre la conexión y recuperación habituales desde la interfaz.

Esta guía reúne la operación manual, los paquetes, la seguridad y los límites actuales. Las direcciones siguientes son **ejemplos ficticios**: origen `192.168.50.10`, receptor `192.168.50.20`, LAN `192.168.50.0/24`, interfaz `wlan0`.

## Paquetes y actualizaciones

`omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` instala el widget. Omarchy no ejecuta hooks de instalación del repositorio. El botón **Instalar agente** abre una terminal de Omarchy y ejecuta `packaging/install-agent.sh`. Instala los paquetes ausentes `rust`, `avahi` y `wl-clipboard` con `omarchy pkg add`, compila el binario release y lo instala en `~/.local/bin`. Registra solo los paquetes que instaló en `~/.local/state/seamlesscontrol/installed-packages`. Avahi se activa cuando hace falta para el descubrimiento mDNS. Las cuatro unidades de usuario se instalan, pero quedan deshabilitadas. Los archivos de servicio existentes se conservan si difieren de los incluidos en el plugin.

Alternativa desde terminal:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh
```

Para actualizar el widget en **cada** equipo, ejecuta `omarchy plugin update seamlesscontrol.control` y después `omarchy restart shell` para cargar el QML nuevo. Si cambió el agente, detén cualquier sesión activa, pulsa **Actualizar agente** en el panel (o usa el comando de terminal) y reinicia el agente en ambos extremos. Un proceso en marcha conserva la revisión anterior hasta reiniciarlo. Las claves y el mapa se conservan.

**Retirar agente** ejecuta `uninstall-agent.sh --remove-deps` en una terminal de Omarchy. Detiene y retira los archivos de servicio del plugin sin modificaciones, conserva los modificados, elimina el binario y retira solo los paquetes registrados como instalados por este plugin. Pacman rechazará la retirada de paquetes requeridos por otros. Los paquetes que ya estaban presentes no se registran. La desinstalación CLI predeterminada, sin `--remove-deps`, conserva los paquetes. Ninguna ruta borra `~/.config/seamlesscontrol/` (identidad, pares, mapa). Retira una regla de firewall por separado antes de `omarchy plugin remove seamlesscontrol.control`.

## Conexión manual

Inicia el receptor; después busca y empareja desde el equipo con ratón:

```bash
seamlesscontrold serve-auto 47832
seamlesscontrold local-address 47832
seamlesscontrold discover
seamlesscontrold pair 192.168.50.20:47832
```

El primer comando se ejecuta en el receptor; los dos últimos, en el origen. `serve-auto` selecciona una IPv4 según la ruta mDNS y anuncia `_seamlesscontrol._tcp` mediante Avahi. La búsqueda necesita multicast mDNS; si está bloqueado, usa `seamlesscontrold serve 192.168.50.20:47832` y la IP manual. La huella anunciada por mDNS es orientativa. El emparejamiento usa Noise XX y un código de seis cifras que debe compararse y aprobarse **en ambos equipos** desde el panel o con `seamlesscontrold status` y `seamlesscontrold approve <código-de-seis-cifras>`. En el receptor, `status` muestra `serve pairing` mientras espera. `seamlesscontrold reject` cancela. La identidad local de 64 caracteres es una huella pública permanente, no el código de comparación.

Coloca cada equipo junto al otro en los mapas de ambos. En el origen, si el receptor está a la derecha:

```bash
seamlesscontrold topology set 192.168.50.20 1 0
seamlesscontrold connect 192.168.50.20:47832
```

En el receptor, ubícalo a la derecha y al origen a la izquierda:

```bash
seamlesscontrold topology set local 1 0
seamlesscontrold topology set 192.168.50.10 0 0
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

`allow` y `remove` piden confirmación y usan Polkit en sesión gráfica o sudo en terminal. El panel ya mostró la regla y confirmó el clic, así que pasa `--yes`; sigue siendo necesaria la autorización del sistema. El script no activa UFW. Si cambia la IP o subred del receptor, revisa `sudo ufw status numbered` y elimina manualmente reglas antiguas. La recepción de archivos usa normalmente TCP `47833` y necesita una regla aparte si está bloqueada.

## Protocolo y otras funciones

El origen captura entrada mediante el portal del escritorio y EIS. El receptor la inyecta a través de la entrada virtual de Hyprland. Su teclado virtual usa el mapa XKB del compositor para actualizar los modificadores de Wayland con cada tecla, de modo que combinaciones como Super+V puedan llegar a Hyprland como atajos. Las sesiones de red usan Noise XX con claves de pares fijadas. El receptor permite un solo dueño de entrada y libera teclas y botones retenidos al desconectar. El bloqueo se comprueba mediante el IPC de Hyprland; un estado desconocido o bloqueado impide la inyección. Tras Escape o retorno remoto, el puntero debe alejarse 96 píxeles antes de rearmar el borde y evitar una recaptura inmediata.

El portapapeles de texto usa UTF-8, tiene límite de 256 KiB y excluye selecciones sensibles de Wayland. La malla experimental 2×2 usa `seamlesscontrold mesh 47832` en el origen después de emparejar y ubicar todos los receptores en todos los mapas. Autentica cada destino y libera al dueño anterior antes de cambiar al siguiente. Revocar una identidad emparejada cierra su conexión de malla aunque ya no controle la entrada; se descartan sus mensajes pendientes y, si tenía el control, se libera la captura. El portapapeles en malla, los cruces rápidos y la recuperación de red siguen pendientes de validación física con varios equipos.

Los archivos usan otra sesión Noise y aprobación del receptor. En el panel, inicia **Esperar un archivo** en el receptor, elige un equipo emparejado y un archivo en el origen, y aprueba la oferta en el receptor. El puerto de archivos (`47833` por defecto) necesita una regla LAN distinta de la del control; el panel permite prepararla y autorizarla. El panel inicia `seamlesscontrold choose-file en|es` o `choose-folder en|es` como proceso separado; esos comandos consultan al portal del escritorio existente para elegir una ruta local y solo imprimen esa ruta como cadena JSON. No abren el archivo ni inician una transferencia. Los equivalentes CLI para transferir son `receive-file-auto 47833 ~/Downloads` y `send-file 192.168.50.20:47833 /ruta/al/archivo`. Límite predeterminado de 100 MiB; comprueba tamaño y SHA-256 antes de publicar. Nunca sobrescribe archivos. Una transferencia física entre dos Omarchy pasó el 2026-10-02; consulta los [resultados](TEST-RESULTS.es.md).

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

Las pruebas de loopback de emparejamiento, descubrimiento, archivos, reconexión y cambios de IP están en [TESTING.md](TESTING.md). La captura e inyección integradas necesitan una sesión Omarchy real sin otro agente activo. El éxito físico observado cubre dos Omarchy con una disposición concreta de monitores, retorno por el borde, Escape y Super+V. Otros atajos de teclado, archivos, malla de varios equipos, otras disposiciones, suspensión y pérdida de red requieren más pruebas físicas. El [receptor Windows x64 de alpha](WINDOWS-ALPHA.es.md) compila y comparte este protocolo; aún falta probar físicamente Windows↔Omarchy.
