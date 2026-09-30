# SeamlessControl · Plan de producto y desarrollo

> Estado: prototipo Omarchy en validación · 29 de septiembre de 2026
> Primera plataforma: **Omarchy ↔ Omarchy** · Expansión posterior: **Windows ↔ Omarchy**  
> Repositorio público del proyecto: **[PuroDelphi/seamlesscontrol](https://github.com/PuroDelphi/seamlesscontrol)**  
> [Ver el plan visual](./index.html)

**Avance de fase 0:** [inventario y probes en este equipo](./docs/FEASIBILITY.md). El portal aceptó una barrera, abrió EIS y entregó movimiento, teclado y clic físicos al cruzar el borde; Hyprland creó teclado y puntero virtuales. El [agente experimental](./README.md) ya conecta captura, transporte cifrado e inyección, retorno, portapapeles de texto y transferencia explícita de archivos por CLI y panel Omarchy. El receptor puede generar una solicitud de retorno al alcanzar el borde exterior hacia el origen según su mapa y corta la sesión si Hyprland indica bloqueo o deja de informar un estado verificable. El panel inicia y termina sesiones, descubre receptores por mDNS sin pedir su IP y guarda una cuadrícula de hasta cuatro equipos; la CLI calcula rutas por saltos contiguos. Si un par cambia de IP, Noise comprueba su clave guardada antes de trasladar su casilla. El receptor puede elegir automáticamente su IP LAN y la identidad local dispone de rotación explícita. El receptor atiende conexiones independientes y concede la entrada a un solo par; el emisor solicita la reserva antes de abrir el portal. El modo experimental `mesh` conecta varias sesiones cifradas y espera la liberación del dueño anterior antes de ceder la entrada a otro destino; aún requiere validación física y se puede iniciar desde el panel. El comando `latency` mide p50/p95 del viaje cifrado entre pares, sin incluir captura ni inyección. El protocolo de archivos tiene confirmación en el destino, tamaño y SHA-256; el panel incluye selectores de archivo y directorio, pero aún necesita prueba física. Falta demostrar el flujo real entre dos equipos; [pruebas pendientes](./docs/TESTING.md).

## 1. Objetivo

Controlar varios equipos Omarchy de una misma red con un solo teclado y ratón. Al cruzar el borde configurado de una pantalla, el control pasa al otro equipo sin abrir una sesión de escritorio remoto. La experiencia incluye emparejamiento, disposición de pantallas, reconexión y, en fases posteriores de la primera versión, portapapeles y archivos.

La referencia funcional es [Mouse Without Borders de PowerToys](https://learn.microsoft.com/en-us/windows/powertoys/mouse-without-borders): control de hasta cuatro equipos, portapapeles compartido, transferencia de archivos y disposición de equipos. SeamlessControl tendrá un protocolo propio; la compatibilidad directa con el protocolo de Microsoft **no forma parte del alcance**.

## 2. Alcance y secuencia

| Entrega | Funciones | Queda para después |
|---|---|---|
| Prueba técnica | Captura al borde, cesión de foco, inyección de ratón y teclado entre dos sesiones Hyprland | Interfaz pulida, portapapeles, archivos |
| MVP Omarchy | Dos equipos; emparejamiento con código, cifrado, disposición izquierda/derecha, cambio por borde, vuelta al origen, reconexión, pausa y salida de emergencia | Más de dos equipos, archivos |
| V1 Omarchy | Hasta cuatro equipos, cuadrícula 2×2, monitores múltiples, portapapeles de texto, envío explícito de archivos, diagnósticos, instalación y actualización documentadas | Compatibilidad Windows |
| V2 multiplataforma | Agente Windows que implemente el mismo protocolo versionado y se conecte con Omarchy | Integración con el protocolo interno de Mouse Without Borders |

**Criterio de enfoque:** cerrar primero un circuito fiable de teclado y ratón entre dos máquinas reales. El portapapeles y los archivos dependen de ese circuito, pero no deben bloquearlo.

## 3. Experiencia esperada

1. En ambos equipos se instala el agente y el plugin. El panel muestra el nombre local y el estado «Disponible».
2. El usuario inicia el emparejamiento en uno, introduce en el otro un código temporal de un solo uso y confirma las identidades mostradas en ambos.
3. En el panel se arrastran los equipos para reflejar su posición física. El sistema guarda la disposición y los bordes activos.
4. Al cruzar un borde activo, el puntero aparece en el equipo vecino y el teclado sigue al puntero. Una indicación discreta identifica el equipo que recibe el control.
5. Al volver por el borde correspondiente, el control regresa. Un atajo local siempre permite recuperarlo y otro pausa el intercambio.
6. Si la conexión cae, se sueltan todas las teclas y botones remotos, se devuelve el control local y el panel muestra el motivo y la acción de reconectar.

## 4. Arquitectura propuesta

```text
┌──────────────────────── Equipo Omarchy A ────────────────────────┐
│ Plugin Omarchy (QML) ← IPC local → seamlesscontrold (Rust)       │
│ panel / barra / ajustes          captura · estado · transporte    │
│                                  Hyprland / Wayland · portapapeles│
└─────────────────────────────────┬─────────────────────────────────┘
                                  │ red local cifrada
┌─────────────────────────────────┴─────────────────────────────────┐
│ seamlesscontrold (Rust) ← IPC local → Plugin Omarchy (QML)       │
│ inyección · estado · transporte    panel / barra / ajustes        │
└──────────────────────── Equipo Omarchy B ────────────────────────┘
```

- **Agente `seamlesscontrold`:** proceso de usuario por sesión gráfica; conoce los equipos, la topología, la conexión y el dueño actual de la entrada. Mantiene el transporte y las interfaces Wayland/Hyprland. Se ejecuta como servicio `systemd --user` vinculado a la sesión gráfica, sin `root` ni acceso permanente a `/dev/uinput` como requisito inicial.
- **Plugin `seamlesscontrol.control`:** repositorio instalable como plugin de Omarchy, con `manifest.json` y componentes QML para panel y estado de barra. La UI habla con el agente por socket Unix local. Los plugins de terceros viven bajo `~/.config/omarchy/plugins/<id>/`; la interfaz no aloja el motor de red ni las credenciales.
- **Protocolo:** mensajes binarios con longitud, tipo, secuencia y versión; canal cifrado y autenticado por equipo. En el primer prototipo se mide TCP/TLS con eventos de movimiento agrupados y coalescidos. La elección final de transporte se confirma con pruebas de latencia y recuperación; el formato versionado debe permitir un agente Windows posterior.
- **Configuración:** topología y preferencias en el directorio de configuración del agente; identidad y claves privadas en un almacén de usuario con permisos restrictivos. `shell.json` sólo registra la activación y posición del plugin.

La [documentación de plugins de Omarchy](https://github.com/omacom/omarchy/blob/quattro/manual/32-shell-plugins.md) define manifiestos, tipos `panel`/`bar-widget`, instalación y habilitación. La integración exacta QML↔agente y el ciclo de vida del servicio se validarán en la fase de prototipo.

## 5. Decisión técnica crítica: entrada en Wayland

Wayland limita la captura global y la inyección de entrada; no conviene diseñar esta parte como si fuese X11. Hyprland publica [`hyprland_input_capture`](https://github.com/hyprwm/hyprland-protocols) para captura mediante EIS. También existen las interfaces de portal [InputCapture](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.InputCapture.html) y [RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html). Su disponibilidad y permisos efectivos **se deben comprobar en la versión de Omarchy instalada**. Hyprland compila además protocolos de teclado y puntero virtual; eso no garantiza por sí solo un flujo completo y seguro de cesión de control.

**Spike obligatorio, antes de implementar la red:**

1. Inventariar versiones de Omarchy, Hyprland, Quickshell, portal y protocolos expuestos en dos equipos de prueba.
2. Capturar el cruce de un borde y las teclas/botones mientras el control remoto está activo. Verificar si el compositor deja de entregar esos eventos a aplicaciones locales.
3. Inyectar movimiento relativo, clic, rueda, teclas, modificadores y distribución de teclado en el destino. Probar liberación al desconectar.
4. Verificar múltiples monitores, escalado fraccional, pantalla completa, bloqueo/desbloqueo y reconexión de monitor.
5. Registrar qué consentimiento muestra el portal y si una sesión se puede restaurar sin interacción inesperada.

**Salida del spike:** matriz de capacidades y una ruta elegida. En este equipo, el portal `InputCapture` está disponible pero `RemoteDesktop` no; la ruta provisional es portal/libei para captura y protocolos virtuales de Hyprland para inyección. Falta probar el flujo real y los permisos. Si ninguna ruta permite captura y cesión fiables sin privilegios excesivos, replantear el MVP antes de construir el plugin visual.

## 6. Modelo de control y seguridad

- Sólo un equipo es dueño del teclado y ratón compartidos en cada instante. Cada cesión lleva un identificador de sesión y un número de secuencia; se descartan eventos atrasados de una sesión anterior.
- Emparejamiento presencial en ambos equipos con código temporal, confirmación de huella e identidades persistentes. Se negocia una versión de protocolo y se autentican mutuamente los pares en cada conexión.
- Escucha sólo en la red local configurada; descubrimiento local opcional, conexión por IP disponible. No hay exposición a Internet, relay ni apertura automática de puertos.
- Atajo de rescate procesado localmente, pausa visible, revocación de pares y rotación de claves. Bloquear un equipo suspende la inyección y exige revalidar el estado al volver.
- Al perder red o agente: soltar teclas y botones mantenidos, detener el envío, recuperar el puntero local y mostrar el estado. Evitar que un equipo remoto controle la pantalla de bloqueo durante la primera versión.
- Portapapeles: inicialmente sólo `text/plain`, tamaño acotado, sincronización al cambio con marca de origen para impedir bucles. Archivos: envío explícito con confirmación de destino, límite configurable, hash y escritura temporal antes del renombrado final.

## 7. Fases de implementación y puertas de salida

| Fase | Trabajo principal | Entregable verificable | Puerta de salida |
|---|---|---|---|
| 0 · Investigación | Spike de captura/inyección y contratos de plugin; probar dos equipos físicos | Matriz de APIs, riesgos y decisión de ruta | Ratón y teclado funcionan en ambos sentidos sin privilegios de sistema |
| 1 · Núcleo | Máquina de estados, protocolo versionado, cifrado, pares, topología básica, reconexión | Agente ejecutable y CLI de diagnóstico | 30 min de ida/vuelta sin teclas pegadas; caída de red recuperable |
| 2 · Plugin | Manifiesto, panel de pares/topología, indicador de barra, IPC, empaquetado | Plugin instalable en Omarchy y servicio de usuario | Instalar, emparejar, usar, pausar y desinstalar con instrucciones reproducibles |
| 3 · V1 Omarchy | Cuatro equipos, monitores, portapapeles de texto, envío de archivos, accesibilidad y registros | Versión candidata | Matriz de pruebas completa y cero pérdida silenciosa de archivos |
| 4 · Windows | Adaptador de captura/inyección y portapapeles de Windows; empaquetado y pruebas cruzadas | Agente Windows compatible con el protocolo | Casos Omarchy↔Windows en ambos sentidos superan la misma suite funcional |

**Métricas objetivo para la fase 1:** cambio de equipo perceptualmente inmediato (medir p50/p95 en LAN y fijar umbral tras el spike), ninguna tecla o botón retenido tras desconexión, reconexión automática sin nuevo emparejamiento y consumo en reposo medido. No fijar una cifra de latencia antes de medir el hardware y el camino de captura elegidos.

## 8. Casos de prueba que definen «terminado»

- Dos Omarchy con resoluciones, escalas y distribuciones de teclado distintas; movimiento, arrastre, rueda, atajos y modificadores en ambos sentidos.
- Cruce rápido entre bordes, esquinas desactivadas, aplicación a pantalla completa y regreso por atajo de rescate.
- Caída de Wi-Fi, suspensión, bloqueo, reinicio de Hyprland/agente y retiro de un monitor durante una sesión.
- Portapapeles de texto vacío, Unicode, contenido grande y cambios casi simultáneos; sin eco infinito.
- Archivo permitido, archivo rechazado, falta de espacio, interrupción y reintento; comprobación del hash.
- Par revocado no puede reconectar; dispositivo nuevo no puede inyectar entrada sin emparejamiento.
- Instalación y desinstalación sin modificar `/usr/share/omarchy/`, y actualización sin perder pares ni configuración.

## 9. Riesgos y decisiones pendientes

| Riesgo | Mitigación / decisión |
|---|---|
| El portal pide permiso en cada sesión o no permite captura en el borde | Resolver en fase 0; elegir ruta Hyprland específica si es viable, documentar su dependencia |
| Diferencias de keymap, teclas muertas e IME | Transportar códigos de tecla y estado de modificadores; probar keymaps diferentes; definir tratamiento de texto compuesto |
| Coordenadas con varios monitores y escalado | Modelar salidas y bordes en coordenadas lógicas; remapear al cambiar la topología |
| Latencia de TCP o eventos acumulados | Coalescer sólo movimiento; jamás descartar pulsaciones/liberaciones; cambiar transporte si las mediciones lo exigen |
| Plugin con acceso al proceso `omarchy-shell` | Mantener la lógica sensible en el agente y revisar el código QML; Omarchy advierte que los plugins corren sin sandbox de sistema |
| Futura integración Windows | Congelar semántica y pruebas de protocolo antes de crear el adaptador; evitar llamadas a Hyprland en el núcleo común |

## 10. Estructura prevista del repositorio

```text
seamlesscontrol/
├── PLAN.md                    # este plan
├── index.html                 # presentación visual del plan
├── probe/                     # comprobación de capacidades Wayland
├── docs/                      # viabilidad, instalación y diagnóstico
├── agent/                     # daemon, protocolo, adaptadores Omarchy
├── manifest.json              # manifiesto instalable de Omarchy
├── plugin/                    # QML para Omarchy
├── packaging/                 # unidad systemd de usuario e instalador
└── tests/                     # integración y escenarios entre equipos
```

La carpeta `agent/` contiene un prototipo ejecutable; `manifest.json` y `plugin/` contienen un widget instalable con estado, emparejamiento, pausa, revocación y cuadrícula 2×2. La geometría del agente evita capturar los pasos internos entre monitores locales al instalar barreras y recalcula las barreras ante cambios de zonas anunciados por el portal o de la cuadrícula guardada; falta verificar ambos comportamientos en monitores reales. `packaging/` contiene el instalador y cuatro unidades de usuario para receptor manual, receptor con autodescubrimiento, emisor y malla; las pruebas entre equipos siguen en el plan. `~/Work/seamlesscontrol/` es la carpeta local de este proyecto y el repositorio público indicado por el usuario es `https://github.com/PuroDelphi/seamlesscontrol`.

## Fuentes consultadas

- [Microsoft Learn · Mouse Without Borders](https://learn.microsoft.com/en-us/windows/powertoys/mouse-without-borders)
- [Omarchy · manual de plugins de shell](https://github.com/omacom/omarchy/blob/quattro/manual/32-shell-plugins.md)
- [Hyprland protocols · input capture](https://github.com/hyprwm/hyprland-protocols)
- [XDG Desktop Portal · InputCapture](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.InputCapture.html)
- [XDG Desktop Portal · RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
