# Control local del agente

`seamlesscontrold` crea `control.sock` dentro de `$XDG_RUNTIME_DIR/seamlesscontrol/`. El directorio tiene permisos `0700` y el socket `0600`. Sólo se admite un agente por sesión de usuario por ahora. El widget invoca subcomandos del mismo binario; no abre el socket directamente.

En el receptor, el IPC de Hyprland consulta `solitaryBlockedBy` en los monitores. `LOCK` significa bloqueo de sesión; una respuesta ausente, inválida o sin monitor legible se trata como estado indeterminado. El agente no inicia la inyección en ese estado, lo comprueba antes de cada pulsación remota y cierra una sesión activa al detectarlo, lo que desencadena la liberación de entrada retenida. `diagnose` expone `LOCK` para comprobar esta puerta de seguridad.

En el emisor, la fase `locked` suspende la barrera de captura y la sincronización del portapapeles sin perder la conexión. Si había una captura, envía `END` y libera el portal. Sólo reactiva la barrera cuando Hyprland vuelve a informar un estado desbloqueado; la pausa manual sigue teniendo efecto.

| Comando | Efecto |
|---|---|
| `seamlesscontrold status` | Estado en una línea tabulada |
| `seamlesscontrold approve 123456` | Acepta el par pendiente si el código coincide |
| `seamlesscontrold reject` | Rechaza el par pendiente |
| `seamlesscontrold pause` | Desactiva la captura en el emisor y libera el control remoto |
| `seamlesscontrold resume` | Reactiva la barrera de captura |
| `seamlesscontrold return` | En el receptor, solicita al origen que termine la captura activa y recupere el puntero |
| `seamlesscontrold peers` | Enumera pares guardados con IP y clave pública |
| `seamlesscontrold revoke <IP>` | Revoca la clave fijada y corta la sesión activa con esa IP |
| `seamlesscontrold topology` | Enumera las casillas guardadas como líneas `SLOT` |
| `seamlesscontrold topology set <local\|IP> <columna> <fila>` | Sitúa o intercambia un equipo en la cuadrícula 2×2 |
| `seamlesscontrold topology remove <IP>` | Quita un equipo del mapa sin revocar su clave |
| `seamlesscontrold topology route <IP>` | Muestra la ruta mínima por casillas ocupadas como líneas `HOP` |
| `seamlesscontrold mesh <PUERTO>` | Captura desde el origen hacia hasta tres destinos ya emparejados en la cuadrícula |
| `seamlesscontrold diagnose` | Lee por IPC de Hyprland la posición del cursor y los monitores lógicos |

La respuesta `STATUS` contiene, en orden, rol, fase, IP remota, pausa, código pendiente y clave pública remota en hexadecimal. Los campos finales están vacíos fuera de un emparejamiento. El código se muestra durante cinco minutos como máximo; se debe comparar en **ambos equipos** antes de aprobar. La conexión Noise XX no queda disponible para entrada hasta recibir aceptación autenticada de los dos lados.

La fase `reconnecting` indica que el emisor sigue ejecutándose y espera para volver a conectar. El retardo se duplica desde un segundo hasta un máximo de treinta; un error de clave cambiada, revocación o protocolo incompatible termina el intento en vez de repetirlo.

La fase `controlling` ahora aparece en ambos extremos durante una captura. El receptor sólo acepta `return` en esa fase y envía una trama de control autenticada `RETURN` con el identificador de la captura activa por la dirección de vuelta del mismo canal Noise. También puede emitirla al detectar el cursor en el borde exterior que conduce al origen, siempre que el mapa local tenga ese par adyacente. El origen sólo atiende la solicitud si el identificador coincide; responde con `END`, libera la sesión del portal y conserva la conexión para un siguiente cruce. Esta ruta tiene pruebas de geometría y loopback del protocolo; todavía falta verificar el retorno físico entre dos equipos.

Las tramas `Clipboard` son independientes de la captura en el modo `connect` y circulan por ambas direcciones del canal Noise. Contienen la marca `SCB1`, una revisión de 64 bits, los 32 bytes de identidad pública del emisor y un evento `Text` o `Clear`. El receptor exige que la identidad coincida con el par autenticado, texto UTF-8 y un máximo de 256 KiB. La pareja `(revisión, identidad)` ordena los cambios simultáneos; el observador omite el cambio que acaba de aplicar para no devolverlo como eco. `wl-paste --watch` entrega cambios al auxiliar interno `clipboard-helper`, que ignora selecciones sensibles y contenido no textual. El saludo `seamlesscontrol/4` evita interpretar estas tramas con un agente anterior. El modo `mesh` todavía no sincroniza el portapapeles.

Antes de abrir el portal de captura, el emisor envía `CLAIM` por el canal autenticado. El receptor responde `READY` si le concede la única reserva de entrada o `BUSY` si otro par la posee. En este último caso, el emisor reintenta sin solicitar permiso al portal. Las conexiones `PAIR` y `PING` no adquieren esa reserva; el receptor puede procesarlas en trabajadores separados, con un límite de ocho conexiones entrantes. El emparejamiento nuevo se rechaza mientras otro par controla la entrada. Al terminar la conexión, la reserva se libera y se limpian el par y la época activa del estado local.

En modo `mesh`, el origen envía `CLAIM-MESH` a cada destino en una conexión Noise fijada por clave pública; el receptor concede `READY` sólo a una conexión de control. El destino observa sus bordes vecinos y responde `RETURN` hacia el origen físico o `SWITCH\t<IP>` hacia otro destino. El origen valida que la solicitud venga del dueño de la época activa y que el destino sea vecino. Establece la conexión siguiente antes de enviar `RELEASE` al anterior. El receptor rechaza una época distinta, libera todas las teclas y botones retenidos y sólo después envía `ENDED` con esa misma época. Entonces el origen envía `BEGIN` con una época nueva al siguiente destino. Si no llega `ENDED` en dos segundos, termina la sesión y libera el portal; jamás activa al siguiente destino sin el acuse. El `END` usado para volver al control local conserva su comportamiento y no solicita confirmación.

`HandoffCoordinator` mantiene una sola casilla dueña y una cesión pendiente como máximo. Comprueba que la solicitud procede del dueño y la época activos, que el destino es vecino ortogonal y que `ENDED` llega del dueño anterior antes de cambiarlo. El bucle `mesh` lo conecta con las barreras del portal y con varias sesiones cifradas. Los tests recorren cuatro casillas y rechazan pares, épocas o destinos incorrectos; falta una prueba física con varios Omarchy.

La transferencia de archivos usa una conexión Noise XX separada, con las mismas identidades fijadas y el saludo `seamlesscontrol-file/1`. Cada trama `File` tiene secuencia estricta y transporta `Offer(nombre UTF-8, tamaño, SHA-256)`, `Accept`, `Reject`, `Chunk`, `End`, `Complete` o `Cancel`. El receptor solicita confirmación local para cada oferta antes de crear un archivo temporal. Cada bloque mide como máximo 64 KiB, el límite predeterminado es 100 MiB y puede configurarse mediante `SEAMLESSCONTROL_MAX_FILE_BYTES`. El emisor sólo anuncia éxito tras recibir `Complete`, emitido después de comprobar tamaño y hash y publicar el archivo sin reemplazar uno existente. El panel lanza un receptor de una sola conexión mediante `receive-file-ui`; éste anuncia `OFFER` en una línea tabulada y espera `SI` por entrada estándar. La decisión la toma el usuario en el panel. La CLI interactiva `receive-file` conserva el mismo requisito.

`revoke` guarda un marcador de la clave pública antes de eliminar el par por IP y quita su posición guardada. Aunque el equipo vuelva a presentarse o cambie de IP, esa clave no puede ser aceptada de nuevo. Para volver a confiar en ese equipo será necesaria una nueva identidad, cuyo código debe confirmarse en ambos extremos. Si el agente está activo, la revocación se procesa por IPC y libera la entrada remota; sin agente, el comando actualiza el almacén privado directamente.

Los comandos `topology` leen y escriben directamente el archivo privado de configuración, incluso si el agente no está en ejecución. `connect <IP:PUERTO>` calcula el borde desde la casilla contigua del par; `connect <IP:PUERTO> <borde>` mantiene la elección explícita. El IPC todavía es un contrato interno del prototipo: no se expone por red ni sustituye el protocolo binario cifrado entre equipos. Una versión posterior añadirá diagnóstico sin depender de la salida tabulada.

`topology route <IP>` busca el camino más corto por vecinos horizontales o verticales ocupados y exige una clave fijada para cada salto. Una diagonal vacía no cuenta como enlace. El resultado todavía no ordena una cesión de control; define los saltos que necesitará el protocolo de varios equipos.

El panel puede lanzar `serve`, `connect` o `mesh` como proceso hijo de Quickshell. Sólo muestra el botón de terminar para el proceso que lanzó y le envía `SIGINT`, de modo que el agente ejecute su cierre habitual. Las sesiones iniciadas por terminal o `systemd --user` conservan su propio ciclo de vida. El panel usa el mapa 2×2 existente para que `connect` deduzca el borde y `mesh` instale las barreras vecinas.
