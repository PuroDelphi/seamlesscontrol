# Control local del agente

`seamlesscontrold` crea `control.sock` dentro de `$XDG_RUNTIME_DIR/seamlesscontrol/`. El directorio tiene permisos `0700` y el socket `0600`. Sólo se admite un agente por sesión de usuario por ahora. El widget invoca subcomandos del mismo binario; no abre el socket directamente.

| Comando | Efecto |
|---|---|
| `seamlesscontrold status` | Estado en una línea tabulada |
| `seamlesscontrold approve 123456` | Acepta el par pendiente si el código coincide |
| `seamlesscontrold reject` | Rechaza el par pendiente |
| `seamlesscontrold pause` | Desactiva la captura en el emisor y libera el control remoto |
| `seamlesscontrold resume` | Reactiva la barrera de captura |
| `seamlesscontrold peers` | Enumera pares guardados con IP y clave pública |
| `seamlesscontrold revoke <IP>` | Revoca la clave fijada y corta la sesión activa con esa IP |

La respuesta `STATUS` contiene, en orden, rol, fase, IP remota, pausa, código pendiente y clave pública remota en hexadecimal. Los campos finales están vacíos fuera de un emparejamiento. El código se muestra durante cinco minutos como máximo; se debe comparar en **ambos equipos** antes de aprobar. La conexión Noise XX no queda disponible para entrada hasta recibir aceptación autenticada de los dos lados.

`revoke` guarda un marcador de la clave pública antes de eliminar el par por IP. Aunque el equipo vuelva a presentarse o cambie de IP, esa clave no puede ser aceptada de nuevo. Para volver a confiar en ese equipo será necesaria una nueva identidad, cuyo código debe confirmarse en ambos extremos. Si el agente está activo, la revocación se procesa por IPC y libera la entrada remota; sin agente, el comando actualiza el almacén privado directamente.

El IPC todavía es un contrato interno del prototipo: no se expone por red ni sustituye el protocolo binario cifrado entre equipos. Una versión posterior añadirá comandos de topología y diagnóstico sin depender de la salida tabulada.
