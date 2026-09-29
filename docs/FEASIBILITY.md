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

El siguiente experimento debe recibir eventos reales, liberar la captura y repetir el flujo en dos equipos. Ningún dato de entrada fue capturado durante esta comprobación.

## Pendiente para cerrar la fase 0

1. Conseguir una segunda sesión Omarchy física para probar origen y destino.
2. Extender el cliente `InputCapture` para recibir eventos EIS y liberar la captura con un atajo local.
3. Enviar eventos de prueba al teclado/puntero virtual del segundo equipo y verificar liberación de teclas y botones.
4. Medir cruce, vuelta, desconexión y pantalla completa; documentar resultados y límites.
5. Integrar ambos adaptadores en el agente y medir la latencia real.
