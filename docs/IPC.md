# Control local del agente

`seamlesscontrold` crea `control.sock` dentro de `$XDG_RUNTIME_DIR/seamlesscontrol/`. El directorio tiene permisos `0700` y el socket `0600`. Sólo se admite un agente por sesión de usuario por ahora. El widget invoca subcomandos del mismo binario; no abre el socket directamente.

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
| `seamlesscontrold diagnose` | Lee por IPC de Hyprland la posición del cursor y los monitores lógicos |

La respuesta `STATUS` contiene, en orden, rol, fase, IP remota, pausa, código pendiente y clave pública remota en hexadecimal. Los campos finales están vacíos fuera de un emparejamiento. El código se muestra durante cinco minutos como máximo; se debe comparar en **ambos equipos** antes de aprobar. La conexión Noise XX no queda disponible para entrada hasta recibir aceptación autenticada de los dos lados.

La fase `reconnecting` indica que el emisor sigue ejecutándose y espera para volver a conectar. El retardo se duplica desde un segundo hasta un máximo de treinta; un error de clave cambiada, revocación o protocolo incompatible termina el intento en vez de repetirlo.

La fase `controlling` ahora aparece en ambos extremos durante una captura. El receptor sólo acepta `return` en esa fase y envía una trama de control autenticada `RETURN` con el identificador de la captura activa por la dirección de vuelta del mismo canal Noise. También puede emitirla al detectar el cursor en el borde exterior que conduce al origen, siempre que el mapa local tenga ese par adyacente. El origen sólo atiende la solicitud si el identificador coincide; responde con `END`, libera la sesión del portal y conserva la conexión para un siguiente cruce. Esta ruta tiene pruebas de geometría y loopback del protocolo; todavía falta verificar el retorno físico entre dos equipos.

`revoke` guarda un marcador de la clave pública antes de eliminar el par por IP y quita su posición guardada. Aunque el equipo vuelva a presentarse o cambie de IP, esa clave no puede ser aceptada de nuevo. Para volver a confiar en ese equipo será necesaria una nueva identidad, cuyo código debe confirmarse en ambos extremos. Si el agente está activo, la revocación se procesa por IPC y libera la entrada remota; sin agente, el comando actualiza el almacén privado directamente.

Los comandos `topology` leen y escriben directamente el archivo privado de configuración, incluso si el agente no está en ejecución. `connect <IP:PUERTO>` calcula el borde desde la casilla contigua del par; `connect <IP:PUERTO> <borde>` mantiene la elección explícita. El IPC todavía es un contrato interno del prototipo: no se expone por red ni sustituye el protocolo binario cifrado entre equipos. Una versión posterior añadirá diagnóstico sin depender de la salida tabulada.
