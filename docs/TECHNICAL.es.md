# Guía técnica de SeamlessControl

[Guía de usuario](../README.es.md) · [English](TECHNICAL.md) · [Resultados de pruebas físicas](TEST-RESULTS.es.md) · [Registro detallado](FEASIBILITY.md) · [Plan de pruebas](TESTING.md) · [Protocolo local](IPC.md)

Esta guía reúne la operación manual, los paquetes, la seguridad y los límites actuales. Las direcciones siguientes son **ejemplos ficticios**: origen `192.168.50.10`, receptor `192.168.50.20`, LAN `192.168.50.0/24`, interfaz `wlan0`.

## Paquetes y actualizaciones

`omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` instala el widget. Omarchy no ejecuta hooks de instalación del repositorio. El botón **Instalar agente** abre una terminal de Omarchy y ejecuta `packaging/install-agent.sh`. Instala los paquetes ausentes `rust`, `avahi` y `wl-clipboard` con `omarchy pkg add`, compila el binario release y lo instala en `~/.local/bin`. Registra solo los paquetes que instaló en `~/.local/state/seamlesscontrol/installed-packages`. Avahi se activa cuando hace falta para el descubrimiento mDNS. Las cuatro unidades de usuario se copian, pero quedan deshabilitadas.

Alternativa desde terminal:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh
```

Para actualizar el widget en **cada** equipo, ejecuta `omarchy plugin update seamlesscontrol.control`. Si cambió el agente, pulsa otra vez **Instalar agente** (o usa el comando de terminal) y reinicia los agentes activos en ambos extremos. Un proceso en marcha conserva la revisión anterior hasta reiniciarlo. Las claves y el mapa se conservan.

**Retirar agente** ejecuta `uninstall-agent.sh --remove-deps` en una terminal de Omarchy. Detiene los servicios de usuario instalados, elimina el binario y retira solo los paquetes registrados como instalados por este plugin. Pacman rechazará la retirada de paquetes requeridos por otros. Los paquetes que ya estaban presentes no se registran. La desinstalación CLI predeterminada, sin `--remove-deps`, conserva los paquetes. Ninguna ruta borra `~/.config/seamlesscontrol/` (identidad, pares, mapa). Retira una regla de firewall por separado antes de `omarchy plugin remove seamlesscontrol.control`.

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

El origen captura entrada mediante el portal del escritorio y EIS. El receptor la inyecta a través de la entrada virtual de Hyprland. Las sesiones de red usan Noise XX con claves de pares fijadas. El receptor permite un solo dueño de entrada y libera teclas y botones retenidos al desconectar. El bloqueo se comprueba mediante el IPC de Hyprland; un estado desconocido o bloqueado impide la inyección. Tras Escape o retorno remoto, el puntero debe alejarse 96 píxeles antes de rearmar el borde y evitar una recaptura inmediata.

El portapapeles de texto usa UTF-8, tiene límite de 256 KiB y excluye selecciones sensibles de Wayland. La malla experimental 2×2 usa `seamlesscontrold mesh 47832` en el origen después de emparejar y ubicar todos los receptores en todos los mapas. Autentica cada destino y libera al dueño anterior antes de cambiar al siguiente. El portapapeles en malla, los cruces rápidos y la recuperación de red siguen pendientes de validación física con varios equipos.

Los archivos usan otra sesión Noise y aprobación del receptor. En el panel, inicia **Esperar un archivo** en el receptor, elige un equipo emparejado y un archivo en el origen, y aprueba la oferta en el receptor. Los equivalentes CLI son `receive-file-auto 47833 ~/Downloads` y `send-file 192.168.50.20:47833 /ruta/al/archivo`. Límite predeterminado de 100 MiB; comprueba tamaño y SHA-256 antes de publicar. Nunca sobrescribe archivos. La transferencia física entre dos Omarchy sigue pendiente de prueba.

Se instalan, pero no se activan, estas unidades: `seamlesscontrol-receiver-auto.service`, `seamlesscontrol-receiver.service`, `seamlesscontrol-sender.service` y `seamlesscontrol-mesh.service`. El receptor automático usa `47832`. Archivos de entorno manuales en `~/.config/seamlesscontrol/` pueden definir `SEAMLESSCONTROL_LISTEN`, `SEAMLESSCONTROL_PEER`, `SEAMLESSCONTROL_EDGE` o `SEAMLESSCONTROL_PORT`. Activa solo la unidad elegida por equipo.

## Verificación y límites

```bash
bash tests/firewall_lan.sh
bash tests/agent_setup_packages.sh
bash tests/uninstall_agent.sh
cargo test --manifest-path agent/Cargo.toml
cargo clippy --manifest-path agent/Cargo.toml --all-targets -- -D warnings
```

Las pruebas de loopback de emparejamiento, descubrimiento, archivos, reconexión y cambios de IP están en [TESTING.md](TESTING.md). La captura e inyección integradas necesitan una sesión Omarchy real sin otro agente activo. El éxito físico observado cubre dos Omarchy con una disposición concreta de monitores y retorno por los bordes derecho e izquierdo. Teclado, archivos, malla de varios equipos, otras disposiciones, suspensión y pérdida de red requieren más pruebas físicas. Windows sigue pendiente.
