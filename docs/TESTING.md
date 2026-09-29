# Pruebas de dos equipos · pendiente de hardware

El agente experimental compila y supera las pruebas locales, pero **aún no se ha validado entre dos Omarchy** porque sólo hay uno disponible. Esta guía define la primera prueba reproducible y evita confundir simulaciones de loopback con una prueba de entrada física.

La prueba automatizada `bash tests/pair_loopback.sh` sí comprueba dos identidades separadas, Noise XX, comparación del código, aprobación por IPC, reconexión con claves fijadas, persistencia e intercambio de posiciones de la cuadrícula, revocación, limpieza de la posición y rechazo posterior, además del cierre del receptor. `bash tests/reconnect_wait.sh` comprueba el reintento del emisor sin receptor y su salida limpia. Ninguna abre el portal ni inyecta entrada.

## Preparación

1. Instalar la misma revisión del repositorio en ambos equipos Omarchy y compilar el binario de release indicado en el README.
2. Registrar `omarchy --version`, `hyprctl version`, `quickshell --version`, versión del portal, monitores (`hyprctl monitors`) y distribución de teclado en ambos.
3. Confirmar conectividad LAN y que el puerto elegido acepta TCP únicamente desde la LAN. No abrirlo hacia Internet.
4. Mantener un terminal accesible en cada máquina. Al primer emparejamiento, comparar el código de seis cifras en ambos paneles o terminales antes de aprobar en ambos. Por CLI: `seamlesscontrold status` y `seamlesscontrold approve <código>`.

## Circuito básico

1. Ejecutar `serve` en B y `connect ... right` en A. Comprobar que el portal de A solicita el permiso esperado y que se autentica el mismo par.
2. Llevar el ratón **físico** al borde derecho de A. Moverlo en B, hacer clic, arrastrar, usar la rueda y escribir con letras, modificadores y atajos. Confirmar que esos eventos no actúan también sobre aplicaciones de A.
3. Pulsar Escape mientras B tiene el control. Confirmar que A recupera el puntero y que B no conserva teclas ni botones pulsados.
4. Interrumpir la red mientras se mantiene una tecla y luego un botón. Confirmar que B los libera al vencer el plazo de latido y que A recupera el control.
5. Repetir cambiando A y B, y con los cuatro bordes. Registrar p50/p95 del tiempo desde el cruce hasta el primer movimiento o tecla visible en destino.

## Casos que deben pasar antes de cerrar la fase 0

- Ratón, teclado, modificadores, rueda y arrastre en ambos sentidos, sin duplicación local.
- Cruces rápidos, esquinas y pantalla completa; salida de emergencia bajo pérdida de red.
- Monitores múltiples, escala fraccional, distinta resolución y distinto mapa de teclado.
- En A con dos o más monitores, recorrer las uniones entre monitores sin activar el control remoto; cruzar cada tramo exterior del borde elegido y comprobar la activación. Repetir tras cambiar la disposición y después de desconectar un monitor, reiniciando el agente mientras no exista manejo de zonas en vivo.
- Bloqueo, suspensión, reinicio de Hyprland, desconexión de monitor y recuperación.
- Consentimiento del portal en primera conexión y conexiones siguientes.

Anotar en `docs/FEASIBILITY.md` resultados observados, comandos, versiones y límites. Un fallo de captura física o de cesión de foco bloquea la puerta de salida de la fase 0.
