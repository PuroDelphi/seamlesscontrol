# Pruebas de dos equipos · pendiente de hardware

El agente experimental compila y supera las pruebas locales, pero **aún no se ha validado entre dos Omarchy** porque sólo hay uno disponible. Esta guía define la primera prueba reproducible y evita confundir simulaciones de loopback con una prueba de entrada física.

La prueba automatizada `bash tests/pair_loopback.sh` sí comprueba dos identidades separadas, Noise XX, comparación del código, aprobación por IPC, reconexión con claves fijadas, persistencia e intercambio de posiciones de la cuadrícula, revocación, limpieza de la posición y rechazo posterior, además del cierre del receptor. `bash tests/reconnect_wait.sh` comprueba el reintento del emisor sin receptor y su salida limpia. `bash tests/file_loopback.sh` comprueba la oferta que usa el panel, la aceptación con verificación del archivo y el rechazo sin escritura. Ninguna abre el portal ni inyecta entrada.

## Preparación

1. Instalar la misma revisión del repositorio en ambos equipos Omarchy y compilar el binario de release indicado en el README.
2. Registrar `omarchy --version`, `hyprctl version`, `quickshell --version`, versión del portal, monitores (`hyprctl monitors`) y distribución de teclado en ambos.
3. Confirmar conectividad LAN y que el puerto elegido acepta TCP únicamente desde la LAN. No abrirlo hacia Internet.
4. Mantener un terminal accesible en cada máquina. Al primer emparejamiento, comparar el código de seis cifras en ambos paneles o terminales antes de aprobar en ambos. Por CLI: `seamlesscontrold status` y `seamlesscontrold approve <código>`.
5. Ejecutar `seamlesscontrold diagnose` en ambos equipos y comparar sus rectángulos lógicos con `hyprctl -j monitors`; el comando debe mostrar también una posición de cursor dentro de un monitor.
6. Iniciar `serve` y `connect` desde los botones del panel, repetir con terminal y servicio de usuario, y comprobar que el botón **Terminar sesión** sólo aparece para el proceso creado por el panel. Verificar que al terminar no quedan teclado ni botones pulsados ni procesos huérfanos.

## Circuito básico

1. Ejecutar `serve` en B y `connect ... right` en A. Comprobar que el portal de A solicita el permiso esperado y que se autentica el mismo par.
2. Llevar el ratón **físico** al borde derecho de A. Moverlo en B, hacer clic, arrastrar, usar la rueda y escribir con letras, modificadores y atajos. Confirmar que esos eventos no actúan también sobre aplicaciones de A.
3. Pulsar Escape mientras B tiene el control. Confirmar que A recupera el puntero y que B no conserva teclas ni botones pulsados. Repetir con **Devolver control al origen** en el panel de B y con `seamlesscontrold return`; verificar que A libera el portal y que un nuevo cruce vuelve a funcionar.
4. Configurar el mapa inverso en B. Activar el control en B, alejar el cursor al menos 16 píxeles del borde hacia A y cruzar ese borde. Confirmar que A recupera el control una sola vez y que una nueva activación sigue siendo posible. Repetir con monitores escalados y uniones internas, que no deben activar la vuelta.
5. Interrumpir la red mientras se mantiene una tecla y luego un botón. Confirmar que B los libera al vencer el plazo de latido y que A recupera el control.
6. Repetir cambiando A y B, y con los cuatro bordes. Registrar p50/p95 del tiempo desde el cruce hasta el primer movimiento o tecla visible en destino.
7. Con la conexión activa, copiar texto ASCII, Unicode, texto vacío y más de 256 KiB en A y B. Comprobar la sincronización bidireccional, que el texto excesivo no se transmite y que no hay rebotes repetidos. Copiar casi a la vez en ambos extremos y comprobar que terminan con el mismo contenido. Probar una selección marcada como sensible por `wl-clipboard`; no debe llegar al otro equipo. Repetir tras reconectar. Estos casos siguen pendientes de dos máquinas físicas.
8. Después de emparejar, probar el panel **Archivos** en ambos sentidos con otro puerto LAN. Repetir por CLI con `receive-file <IP-B:47833> <directorio>` en B y `send-file <IP-B:47833> <archivo>` en A. Probar aceptación, rechazo, archivo vacío, Unicode, límite de tamaño, archivo existente, destino sin espacio, corte de red y reintento. Comprobar SHA-256 en ambos y que nunca quede un archivo final incompleto ni se reemplace uno existente. Repetir cambiando A y B. Esta prueba física sigue pendiente.

## Casos que deben pasar antes de cerrar la fase 0

- Ratón, teclado, modificadores, rueda y arrastre en ambos sentidos, sin duplicación local.
- Cruces rápidos, esquinas y pantalla completa; salida de emergencia bajo pérdida de red.
- Monitores múltiples, escala fraccional, distinta resolución y distinto mapa de teclado.
- En A con dos o más monitores, recorrer las uniones entre monitores sin activar el control remoto; cruzar cada tramo exterior del borde elegido y comprobar la activación. Repetir tras cambiar la disposición y después de desconectar un monitor **sin reiniciar el agente**, para comprobar que la señal `ZonesChanged` recalcula las barreras y devuelve el control local si el cambio sucede durante una captura.
- Bloqueo, suspensión, reinicio de Hyprland, desconexión de monitor y recuperación.
- Consentimiento del portal en primera conexión y conexiones siguientes.

Anotar en `docs/FEASIBILITY.md` resultados observados, comandos, versiones y límites. Un fallo de captura física o de cesión de foco bloquea la puerta de salida de la fase 0.
