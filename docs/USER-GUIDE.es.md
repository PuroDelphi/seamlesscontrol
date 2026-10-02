# Guía de uso de SeamlessControl

[Volver al README](../README.es.md) · [English](USER-GUIDE.md)

SeamlessControl permite controlar otro Omarchy (el **receptor**) desde el equipo que tiene el ratón y teclado físicos (el **origen**). Primero instala el plugin y el agente en ambos equipos. Todos los pasos siguientes se hacen desde el panel SeamlessControl. Las capturas usan nombres y direcciones ficticios.

## Primera conexión, paso a paso

1. **Inicia el receptor.** En el equipo que quieres controlar, abre SeamlessControl. En **Iniciar sesión**, deja vacía la dirección y pulsa **Recibir control**. La parte superior del panel debe indicar **Disponible**. Deja esta sesión abierta.

   ![Botón Recibir control del receptor](images/firewall-es.png)

2. **Encuéntralo y emparéjalo.** En el origen, busca el receptor en **Equipos en la red**. Pulsa **Buscar** si no aparece. Pulsa **Emparejar** junto al receptor. Aparecerá un código de seis cifras en **ambos** equipos. Compáralo y pulsa **Coincide · aprobar aquí** en cada uno. Solo necesitas emparejar cada equipo de confianza una vez; las sesiones siguientes usan su clave guardada. La **Identidad local**, mucho más larga, es una huella, no el código que debas escribir.

   **Emparejar** autoriza ese receptor una vez. **Conectar** inicia una sesión nueva cada vez que quieras usarlo.

   ![El panel explica Emparejar y Conectar](images/connect-context-es.png)

3. **Ubica los equipos.** En **Mapa de equipos**, coloca el receptor junto a **Este equipo** según su posición real. En el receptor haz lo inverso: coloca el origen junto a **Este equipo**. Pulsa una ficha y luego la casilla de destino, o arrástrala. Con el panel abierto, pulsa Tab para resaltar una casilla (más pulsaciones recorren las cuatro), Enter para seleccionar la ficha, las flechas para llegar a la casilla de destino y Enter otra vez para colocarla. Escape cancela la selección. Este paso solo guarda la dirección del cruce; todavía no inicia el control.

4. **Conecta desde el origen.** En el origen, pulsa **Conectar** junto al receptor descubierto. Espera a que arriba diga **Listo**. El panel indica qué borde debes cruzar. Si el origen tiene varios monitores, usa el **borde exterior de todo el escritorio**, no la separación entre sus monitores. Cruza ese borde con el puntero para entrar al receptor.

   ![Botón Conectar junto a un receptor emparejado](images/connect-button-es.png)

   ![El panel de origen muestra Listo después de iniciar la conexión](images/session-es.png)

5. **Regresa.** Cruza el borde del receptor que mira hacia el origen o pulsa **Escape** en el teclado físico. Si el puntero no vuelve, pulsa **Devolver control al origen** en el panel receptor. Al terminar, pulsa **Terminar sesión iniciada desde el panel** en el origen.

El [README](../README.es.md) explica instalación, actualización y retirada. Emparejar, ubicar y comenzar una sesión son acciones distintas; cambiar solo el mapa no conecta los equipos.

## Si la conexión no avanza

| Lo que muestra el panel | Qué hacer desde el panel |
| --- | --- |
| El receptor no aparece en **Equipos en la red** | Deja **Recibir control** activo en el receptor y pulsa **Buscar** en el origen. Si es un equipo nuevo, escribe su `IP:puerto` en **Emparejar** manual. Si ya está emparejado y ubicado, usa **Conectar por IP**. |
| El emparejamiento agota el tiempo | En el receptor, ve a **Firewall · solo en el receptor**: prepara la regla LAN con el mismo puerto de **Recibir control** y autorízala. Vuelve a **Emparejar** en el origen y aprueba el nuevo código en ambos equipos. |
| **Conectando**, sin conexión de red | Comprueba que **Recibir control** sigue activo en el receptor y que su firewall permite ese puerto. Termina la sesión del origen y vuelve a pulsar **Conectar**. |
| **Red conectada. Preparando la captura del ratón y teclado…** durante más de 15 segundos | En el origen, pulsa **Reiniciar captura de este equipo**. Se cierra el intento atascado y se reinicia el servicio de captura de Omarchy. Esto puede interrumpir otras aplicaciones que compartan pantalla en ese equipo. Cuando el panel indique que terminó, pulsa **Conectar** otra vez. |
| **Listo**, pero el puntero no cruza | Revisa la posición del receptor en el **Mapa de equipos** del origen. Cruza el borde **exterior** indicado de todos sus monitores. Si el panel pide alejar el puntero del borde, muévelo hacia dentro y vuelve a cruzar. |
| **Reconectando** | El agente reintenta por sí solo. Espera un momento y lee **Último intento** para conocer la causa; no inicies una segunda conexión mientras reintenta. Si el receptor se detuvo o cambió su dirección o puerto, inicia allí **Recibir control** y luego pulsa **Terminar sesión iniciada desde el panel** en el origen, **Buscar** y **Conectar** de nuevo. El firewall solo importa si la conexión agota el tiempo antes de autenticarse. |
| El puntero queda en el receptor | Pulsa **Escape** en el teclado del origen o **Devolver control al origen** en el receptor. **Cortar entrada remota · emergencia** desconecta y pausa el receptor; pulsa **Reanudar recepción** antes de volver a probar. |

El botón de reparar la captura solo aparece cuando una sesión iniciada desde el panel ya llegó al receptor, pero la captura del escritorio lleva 15 segundos sin prepararse. Si **Terminar sesión iniciada desde el panel** tampoco responde, el panel envía una señal de parada más fuerte pasados dos segundos.

## Para qué sirve cada función

| Control | Uso |
| --- | --- |
| **Conectar por IP** | Inicia el control cuando el receptor ya está emparejado y ubicado, pero no aparece en el descubrimiento. Escribe su `IP:puerto`. El campo **Emparejar** sirve para confiar en un equipo nuevo, no para iniciar el control. |
| **Portapapeles de texto** | Mientras están conectados, el texto copiado se comparte automáticamente entre los equipos emparejados. No copia archivos ni imágenes. |
| **Conectar varios equipos** | Malla experimental para **un origen y dos o tres receptores** en un mapa 2 × 2 conectado. Empareja y ubica todos los equipos; inicia **Recibir control** en cada receptor con el mismo puerto. Después pulsa este botón en el origen. El texto del portapapeles se distribuye entre los receptores conectados. Con solo dos equipos en total, usa el **Conectar** normal. Falta probar físicamente la malla con más de dos equipos. |
| **Pausar captura / Reanudar captura** | Detiene o reactiva temporalmente la captura al cruzar el borde en el origen, manteniendo la sesión disponible. |
| **Devolver control al origen** | Solicita el regreso desde el receptor mientras está siendo controlado. |
| **Cortar entrada remota · emergencia / Reanudar recepción** | Desconecta la entrada remota en el receptor y bloquea la nueva entrada hasta reanudarla. Úsalo si el origen no logra recuperar el control. |
| **Revocar** | Retira la confianza de un equipo emparejado. Para conectarlo de nuevo habrá que comparar otro código y aprobarlo. |
| **Esperar un archivo / Enviar archivo** | En el receptor, elige una carpeta y pulsa **Esperar un archivo**. Espera a que el panel muestre **Esperando archivo en** con su dirección. Si es el primer envío, pulsa **Preparar regla LAN para archivos**, revisa la regla para el puerto `47833`, pulsa **Autorizar esta regla** y acepta la autorización del sistema. En el origen, elige el receptor emparejado y un archivo, luego **Enviar archivo**. Aprueba la oferta en el receptor. Si eliges otro puerto de archivos, escríbelo también en la dirección del destino en el origen. El límite inicial es 100 MiB y nunca se sobrescriben archivos. |
| **Preparar regla LAN / Autorizar esta regla** | En el receptor, revisa y autoriza una regla de firewall limitada a la red local detectada y al puerto TCP elegido. El control remoto y la recepción de archivos usan puertos distintos y necesitan reglas separadas si el firewall los bloquea. |
| **Actualizar agente / Retirar agente** | Instala la versión actual del agente o la retira junto con los paquetes instalados específicamente por SeamlessControl. Detén primero las sesiones activas. Conserva las claves de emparejamiento y el mapa. |

La [guía técnica](TECHNICAL.es.md) describe el protocolo, el diagnóstico manual y los límites de las pruebas.
