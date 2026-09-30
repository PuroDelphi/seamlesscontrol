# Fase 0 · Comprobación de viabilidad en este equipo

Fecha: 29 de septiembre de 2026. Equipo: sesión Omarchy actual. El segundo equipo aún no ha sido probado.

## Inventario observado

| Componente | Resultado local |
|---|---|
| Omarchy | `4.0.4-1` |
| Hyprland | `0.56.2` |
| Quickshell | `0.3.1` |
| xdg-desktop-portal | `1.22.1-2` |
| xdg-desktop-portal-hyprland | `1.4.1-2` |
| libei | `1.6.0-1` |
| Wayland | `1.26.0-1` |
| Toolchain | C (`cc`) y cabeceras Wayland/libei disponibles; Rust estable 1.98.1 instalado con `mise` para el desarrollo |

## Interfaces anunciadas por Wayland

El [probe de sólo lectura](../probe/README.md) se compiló con `-Wall -Wextra -Werror` y enumeró 71 interfaces. Para SeamlessControl importan:

| Interfaz | Versión | Uso previsto |
|---|---:|---|
| `hyprland_input_capture_manager_v1` | 1 | Captura de entrada mediante EIS |
| `zwlr_virtual_pointer_manager_v1` | 2 | Inyección de puntero en el destino |
| `zwp_virtual_keyboard_manager_v1` | 1 | Inyección de teclado en el destino |
| `zwlr_data_control_manager_v1` | 2 | Observación del portapapeles |
| `ext_data_control_manager_v1` | 1 | Alternativa estándar para portapapeles |

## Portales

- `org.freedesktop.portal.InputCapture` está expuesto, versión `1`; anuncia `SupportedCapabilities = 3` (teclado y puntero). Incluye `CreateSession2`, `SetPointerBarriers` y `ConnectToEIS`.
- `org.freedesktop.portal.RemoteDesktop` **no está expuesto** en esta sesión. Por ello no se debe basar la inyección del MVP en ese portal.
- El descriptor de `xdg-desktop-portal-hyprland` declara `InputCapture` y no `RemoteDesktop`.

## Decisión provisional

Usar `InputCapture`/EIS para capturar en el origen y protocolos de teclado/puntero virtuales de Hyprland para inyectar en el destino. El agente debe aislar ambas detrás de interfaces de plataforma; Windows utilizará otros adaptadores cuando llegue su fase.

La decisión se mantiene **provisional** aunque el probe ya recibe movimiento físico: faltan cesión de foco, calidad de movimiento, permisos repetidos y salida de emergencia en el agente integrado y entre dos equipos.

El 29 de septiembre se ejecutaron además dos probes del agente Rust:

- `virtual-input-probe` creó dispositivos virtuales de teclado y puntero en Hyprland y salió sin enviar eventos.
- `capture-probe` obtuvo permiso para teclado y puntero, instaló una barrera en el borde derecho del primer monitor y abrió una conexión EIS. Terminó sin activar ni registrar eventos de entrada.

Con `capture-probe --listen` y movimiento generado por `virtual-input-probe --edge-test`, la barrera **se activó** y luego se liberó correctamente. El lector no recibió eventos EIS de entrada en los cinco segundos siguientes. Una explicación posible era que el portal no reenviase eventos del dispositivo virtual que activó la barrera; la prueba física posterior tampoco recibió eventos, así que esa explicación no basta. El puntero volvió a su posición original al terminar.

Se repitió el probe después de enlazar las capacidades del asiento EIS *mientras esperaba* la activación, igual que el agente principal en esa revisión. La barrera volvió a activarse con el puntero virtual, pero tampoco llegó un evento EIS durante los cinco segundos posteriores. Enlazar el asiento durante la espera no bastaba.

Con el usuario moviendo el ratón físico al borde derecho durante `cargo run --quiet --bin capture-probe -- --listen`, el portal aceptó la barrera y EIS, notificó **Barrera activada** y el probe liberó la captura tras cinco segundos sin un evento EIS de movimiento, teclado, botón o desplazamiento. La prueba duró menos de los veinte segundos máximos porque el temporizador de lectura empezó al activarse. Esto verifica la activación física y la liberación, pero **no verifica la recepción de entrada capturada**. Hay que averiguar qué eventos y capacidades anuncia el servidor EIS antes de considerar viable el control entre equipos.

Una segunda ventana con ratón físico y un probe que distingue anuncios EIS no activó la barrera en veinte segundos; sí anunció un asiento. En una ejecución controlada posterior, `virtual-input-probe --edge-test` activó la barrera y EIS anunció un asiento y tres dispositivos después de la activación, pero ningún evento de movimiento, teclado, botón o desplazamiento llegó en los cinco segundos de lectura. El probe también reconocía movimiento absoluto. Estos resultados apuntaron al orden de inicialización.

Al esperar `SeatAdded`, enlazar sus capacidades y vaciar la conexión **antes de `InputCapture.Enable`**, el probe recibió anuncios de inicio de captura y un evento EIS en la prueba con puntero virtual. Se aplicó ese mismo orden a `connect` y `mesh` en `seamlesscontrold`. Una nueva prueba con el ratón físico activó la barrera, anunció dos dispositivos y su inicio de captura, y entregó **movimiento relativo**; el probe liberó el cursor. La captura física local de movimiento queda demostrada en este Omarchy. Todavía falta probar teclado físico, el agente integrado y el flujo entre dos equipos. Véase [la guía de pruebas](./TESTING.md).

## Carga real del plugin en Omarchy

El 29 de septiembre se instaló `seamlesscontrol.control` desde GitHub con `omarchy plugin add ... --enable`, y el agente de esta revisión se compiló e instaló en `~/.local/bin` mediante `packaging/install-agent.sh`. La barra mostró el widget. El IPC `omarchy-shell shell summon seamlesscontrol.control '{}'` abrió su panel y `hide` lo cerró; la nueva instancia de Quickshell no registró errores QML del plugin. La primera prueba reveló que el widget no exponía `opened`, propiedad que usa Omarchy para localizar paneles de la barra; quedó corregida en el código. Tras actualizar el plugin, fue necesario `omarchy restart shell` para que la instancia en ejecución cargara la nueva propiedad.

`seamlesscontrold diagnose` leyó `CURSOR 1046 440`, un monitor lógico `0 0 1366 768` y `LOCK unlocked`. No había sesión del agente ni segundo equipo durante esta prueba; abrir el panel no valida captura, inyección ni transporte de entrada.

La revisión 0.14 del widget separa «sin agente» (ejecutable ausente) de «sin sesión» (ejecutable disponible, agente detenido). Se ocultó el binario instalado durante 12 segundos y luego se restauró: el panel cambió a «sin agente», limpió los equipos y la cuadrícula, y volvió a «sin sesión» al detectarlo de nuevo. Durante los segundos anteriores a la siguiente comprobación periódica, Quickshell aún registró intentos fallidos; la cadencia final se redujo a tres segundos. La prueba visual confirmó que ambos estados conservan el estilo del shell.

El diagnóstico `latency` se probó en `127.0.0.1` con dos identidades temporales: rechazó un equipo sin emparejar y, después de aprobar el par, completó veinte solicitudes y respuestas cifradas con mínimo, p50, p95 y máximo ordenados. La prueba también confirmó la revocación posterior. Estos números de loopback no miden la LAN ni el tiempo de captura e inyección; hay que tomar medidas reales en dos Omarchy.

El receptor ahora atiende conexiones entrantes en trabajadores independientes, con un límite de ocho. En loopback, una conexión TCP que no completó el saludo permaneció abierta mientras otro par completó veinte pulsos cifrados; antes, la escucha serial habría quedado bloqueada. Una prueba del protocolo Noise verificó que un segundo `CLAIM` recibe `BUSY` cuando otro par tiene la reserva de entrada. No se inyectó entrada durante esas pruebas. En esa revisión, el saludo de aplicación pasó a `seamlesscontrol/4` para el modo de malla; la revisión actual usa `/5` y debe instalarse en todos los extremos.

Se añadió el intercambio `RELEASE`/`ENDED` como condición para cambiar de destino: una prueba del núcleo verificó que una época antigua no se confirma y que el acuse se produce después de soltar una tecla y un botón retenidos. El receptor Omarchy envía el acuse por el canal Noise; el emisor `mesh` mantiene varias conexiones y lo exige antes de activar otro destino. Todavía falta verificar el cambio del puntero entre equipos físicos.

La política pura `HandoffCoordinator` pasó pruebas de una ruta de cuatro casillas y de solicitudes con origen, época y destino inválidos. Conserva al dueño anterior hasta recibir el acuse correspondiente; un fallo puede devolver la propiedad al equipo físico. La ruta está conectada al portal, EIS y Noise, pero estas pruebas sólo verifican la lógica de propiedad, no la transición real del puntero ni la red entre tres o cuatro máquinas.

Una prueba adicional de loopback abrió dos canales Noise XX con tres identidades fijadas. B recibió una tecla y un botón retenidos, solicitó el cambio a C, liberó ambos y emitió `ENDED`; sólo después el origen activó C con otra época y reprodujo allí las pulsaciones. Con ella, la suite contiene 55 pruebas de Rust. Este ensayo comprueba el orden de tramas y la limpieza de entrada en el receptor. No prueba el portal ni el movimiento de un ratón físico entre escritorios.

Para que el cursor aparezca en el borde vecino, el protocolo de aplicación pasó a `seamlesscontrol/5`. `BEGIN`, `SWITCH` y el retorno automático llevan el borde y una posición relativa de 16 bits. Las 58 pruebas de Rust y Clippy pasan; entre ellas, pruebas puras verifican el remapeo entre 1920×1080 y 1280×720 y que el punto de llegada permanezca dentro de un monitor cuando hay dos salidas escalonadas. En este Omarchy, un movimiento virtual de +4 píxeles desplazó `CURSOR` de `(1233,211)` a `(1237,211)`, y −4 lo devolvió a `(1233,211)`. Un segundo ensayo lo desplazó de `(1233,211)` a `(2,211)` y luego lo restauró a `(1233,211)`. Estas observaciones locales sustentan el movimiento relativo usado para entrar en el destino; falta verificar la posición durante un cruce real entre equipos.

Los modos `mesh` y `connect` con borde deducido ahora comparan cada segundo la topología privada guardada. Si cambia, cierran la captura y vuelven a crearla con el mapa nuevo; un borde indicado explícitamente a `connect` conserva su valor. La recarga compila y supera las pruebas de Rust y Clippy, pero todavía falta cambiar la cuadrícula durante una captura física para verificar el resultado visible.

El centro del portapapeles de malla pasó pruebas con tres identidades: reenvió un cambio de B a C como mensaje autenticado del origen, evitó el eco hacia B, rechazó una identidad suplantada y limitó el tamaño del texto. El agente lo conectó a los canales Noise y al observador Wayland; `cargo test` pasó 54 pruebas de Rust, y las tres pruebas de integración en loopback de emparejamiento, reconexión y archivos conservaron su resultado. La convergencia del portapapeles con copias simultáneas en escritorios reales sigue pendiente.

El modo `mesh` ahora reinicia la sesión de captura después de un fallo de transporte: su cierre libera el portal y las conexiones, informa `reconnecting` por IPC y espera de 1 a 30 segundos antes de crear otro portal. Las validaciones de configuración y claves al principio del intento siguen siendo errores definitivos. Las 54 pruebas de Rust y Clippy pasan tras el cambio; la recuperación del puntero tras una caída de red real sigue pendiente de varios equipos.

El instalador añadió `seamlesscontrol-mesh.service` como unidad de usuario deshabilitada. `systemd-analyze verify` aceptó las tres unidades y `bash -n` aceptó el instalador. La unidad requiere `mesh.env` con el puerto común de los destinos y no se habilitó durante estas pruebas.

En `connect` y `mesh`, la señal de cambio de zonas ahora intenta liberar explícitamente la activación del portal y devuelve el puntero al interior del borde antes de instalar las barreras nuevas. Si el portal ya desactivó la captura, el agente continúa con `disable` y la reinstalación. Este caso sigue pendiente de un cambio real de monitores durante una captura.

La transferencia de archivos ahora usa el mismo descriptor para calcular SHA-256 y leer los fragmentos. Si el archivo origen crece después de la oferta, el emisor envía `Cancel` y el receptor borra el temporal sin publicar el archivo. Pasaron la prueba unitaria y la prueba de sesión cifrada con un origen que crece durante la aprobación. Esto evita publicar silenciosamente el prefijo antiguo; aún falta probar cambios del archivo y cortes de red entre dos equipos físicos.

## Pendiente para cerrar la fase 0

1. Conseguir una segunda sesión Omarchy física para probar origen y destino.
2. Probar teclado físico, liberación mediante un atajo local y captura del agente integrado; el probe ya recibió movimiento relativo físico y liberó el cursor.
3. Enviar eventos de prueba al teclado/puntero virtual del segundo equipo y verificar liberación de teclas y botones.
4. Medir cruce, vuelta, desconexión y pantalla completa; documentar resultados y límites.
5. Medir la latencia real del agente integrado; los adaptadores están conectados en el prototipo, pero todavía no se ha verificado su flujo físico extremo a extremo.
