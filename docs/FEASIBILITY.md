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

La decisión se mantiene **provisional** porque la enumeración no prueba cesión de foco, calidad de movimiento, permisos repetidos ni salida de emergencia.

El 29 de septiembre se ejecutaron además dos probes del agente Rust:

- `virtual-input-probe` creó dispositivos virtuales de teclado y puntero en Hyprland y salió sin enviar eventos.
- `capture-probe` obtuvo permiso para teclado y puntero, instaló una barrera en el borde derecho del primer monitor y abrió una conexión EIS. Terminó sin activar ni registrar eventos de entrada.

Con `capture-probe --listen` y movimiento generado por `virtual-input-probe --edge-test`, la barrera **se activó** y luego se liberó correctamente. El lector no recibió eventos EIS de entrada en los cinco segundos siguientes. Una explicación posible es que el portal no reenvíe eventos del dispositivo virtual que activó la barrera; esto no está demostrado. El puntero volvió a su posición original al terminar. Hace falta repetir la prueba con ratón/teclado físicos y registrar los eventos recibidos.

El siguiente experimento debe recibir eventos reales, liberar la captura y repetir el flujo en dos equipos. Ningún dato de entrada fue capturado durante esta comprobación. El agente experimental `seamlesscontrold` ya enlaza portal/EIS, Noise XX y entrada virtual; está compilado y sometido a pruebas locales, pero su flujo de entrada físico y la ejecución entre dos máquinas siguen sin verificar. Véase [la guía de pruebas](./TESTING.md).

## Pendiente para cerrar la fase 0

1. Conseguir una segunda sesión Omarchy física para probar origen y destino.
2. Probar el cliente EIS con movimiento y teclado físicos; confirmar liberación mediante un atajo local.
3. Enviar eventos de prueba al teclado/puntero virtual del segundo equipo y verificar liberación de teclas y botones.
4. Medir cruce, vuelta, desconexión y pantalla completa; documentar resultados y límites.
5. Medir la latencia real del agente integrado; los adaptadores están conectados en el prototipo, pero todavía no se ha verificado su flujo físico extremo a extremo.
