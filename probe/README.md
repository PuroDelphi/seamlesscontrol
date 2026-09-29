# Probe de capacidades Wayland

Este programa sólo enumera las interfaces que el compositor anuncia a un cliente Wayland. No captura teclado, no mueve el ratón y no modifica la configuración.

## Compilar y ejecutar

```bash
cd ~/Work/seamlesscontrol
cc -Wall -Wextra -Werror -std=c11 probe/wayland_registry.c \
  -o /tmp/seamlesscontrol-wayland-registry \
  $(pkg-config --cflags --libs wayland-client)
/tmp/seamlesscontrol-wayland-registry
```

Ejecutar desde la sesión gráfica de Omarchy. Para repetir la prueba en otro equipo, copiar el código fuente y ejecutar los mismos comandos allí. El resumen indica si aparecen las cuatro interfaces de interés, pero **presencia no implica que un flujo de captura e inyección funcione**.

El portal se puede inspeccionar con:

```bash
busctl --user --no-pager introspect \
  org.freedesktop.portal.Desktop \
  /org/freedesktop/portal/desktop \
  org.freedesktop.portal.InputCapture
```

La prueba completa posterior tendrá que crear una sesión de captura con consentimiento, definir barreras en los bordes, conectarse a EIS e inyectar eventos por teclado y puntero virtuales en una segunda máquina.
