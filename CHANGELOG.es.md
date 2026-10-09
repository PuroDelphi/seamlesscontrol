# Notas de versiones

[English](CHANGELOG.md) · [Todos los releases de GitHub](https://github.com/PuroDelphi/seamlesscontrol/releases)

Este archivo resume los cambios publicados y los cambios aún sin release. Consulta los [resultados de pruebas](docs/TEST-RESULTS.es.md) para conocer las verificaciones y los casos que faltan.

## [0.23.4](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.4) — 2026-10-09

- Vincula la aprobación temporal de archivos manuales y copiados a la identidad autenticada del remitente. Un equipo nuevo con la misma IP debe recibir su propia aprobación; revocar un par borra su permiso temporal.
- Los modos de aceptación automática y de preguntar siempre conservan su funcionamiento. Hay que actualizar juntos el plugin y el agente en Omarchy, o ambos ejecutables en Windows, porque la línea interna de oferta ahora incluye la identidad. Los protocolos cifrados de transferencia y emparejamiento no cambian.
- Añade una prueba cifrada local que revoca y vuelve a emparejar otra identidad en la misma IP, y actualiza las guías de usuario.

## [0.23.3](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.3) — 2026-10-09

- Actualiza las dependencias fijadas Tokio de 1.53.1 a 1.53.2 y `mdns-sd` de 0.21.4 a 0.21.5. La segunda corrige los temporizadores del descubrimiento tras cambios de hora o suspensión; no cambia el código de la aplicación ni el protocolo de emparejamiento.
- Reorganiza README y guías Omarchy/Windows en español e inglés por tareas: instalación, primera conexión, regreso, archivos, permisos y resolución de problemas.
- Renueva las imágenes explicativas desde las interfaces del proyecto, con datos ficticios y procedencia identificada. Separa las instrucciones actuales de las notas históricas de versiones.
- Corrige la configuración innecesaria del mapa inverso en sesiones directas, explica los dos pasos de Conectar en Windows y distingue los puertos de copiar/pegar y envío manual. Retira el enlace al plan inexistente y actualiza la documentación técnica de retorno y parada.
- Simplifica el aviso de las imágenes Windows: muestra la interfaz real con datos de ejemplo, sin referencias al sistema utilizado para generarlas.

## [0.23.2](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.2) — 2026-10-07

- El receptor de control Windows admite como máximo ocho conexiones simultáneas. Las conexiones TCP sobrantes se cierran antes de crear un hilo, para que un equipo no autenticado no pueda aumentar sin límite el consumo de hilos y sockets.
- El inicio del intercambio cifrado Noise en Windows tiene un tiempo de espera de E/S de diez segundos. El plazo largo para emparejar solo empieza después del intercambio criptográfico; la persona conserva tiempo para comparar y aprobar el código de seis cifras.
- Se añade una prueba Windows del límite de conexiones y de la liberación automática de cupos. Esto corrige el problema señalado por el marketplace para 0.23.1; el panel Omarchy y los flujos de usuario no cambian.

## [0.23.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.1) — 2026-10-07

- Los controles no disponibles de Omarchy se atenúan para distinguir visualmente su estado sin cambiar el tema. Se explica por qué **Actualizar agente** no está disponible durante una sesión activa.
- El instalador del agente Omarchy se ejecuta desde la carpeta del plugin para que Cargo y rustc elijan la misma versión de Rust al iniciarlo desde el panel.
- El Omarchy de origen permanece despierto mientras controla otro equipo. El agente libera la captura si se bloquea el origen y aplaza las barreras mientras la geometría del monitor no está disponible tras apagarse la pantalla.
- Se añade un registro de entrada opcional para investigar cortes intermitentes del teclado remoto. En una sesión física entre dos Omarchy, las teclas funcionaron durante el periodo observado; el fallo intermitente no se reprodujo.
- En esa sesión física, el origen siguió desbloqueado tras más de cinco minutos sin entrada física mientras el control permanecía activo. Al cerrar la sesión de diagnóstico, el usuario confirmó que funcionaban el ratón y el teclado físicos. El bloqueo manual durante el control remoto no se ha verificado con esta revisión.

## [0.23.0](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.0) — 2026-10-06

- Se añade **Revocar** a la lista de equipos emparejados de Windows, con confirmación y cierre de una sesión activa.
- Una petición explícita de **Emparejar** muestra un código nuevo de seis cifras en ambos equipos, incluso tras revocar. Las conexiones de control ordinarias siguen bloqueadas hasta que los dos usuarios aprueban el código. Ambas interfaces muestran junto al botón indicaciones sobre el receptor y el firewall.
- El puntero regresa de Omarchy a Windows al cruzar el borde de entrada aunque el receptor Omarchy no tenga una posición guardada para Windows. Verificado en una prueba física de ida y vuelta.
- Las dos interfaces ordenan el emparejamiento y la ubicación en cuatro pasos numerados: emparejar por dirección, equipos cercanos, equipos emparejados y mapa. Los equipos emparejados quedan justo encima del mapa, con instrucciones para ubicarlos.
- El receptor Omarchy puede detenerse normalmente desde Inicio o junto al paso Emparejar, incluso después de reiniciar el shell. El corte de emergencia sigue siendo una pausa; la interfaz explica la diferencia.
- En Windows, Emparejar interrumpe temporalmente la sesión de control de esta app y restaura el modo anterior al terminar. El receptor del otro equipo debe seguir disponible. Así, un agente local ya activo no bloquea el emparejamiento.

## [0.22.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.22.1) — 2026-10-05

- Rechaza nombres de archivo relativos a una unidad Windows, como `C:payload.txt`, nombres de flujos alternativos con `:` y cualquier nombre que no sea un único componente de ruta antes de formar el destino. Una oferta autenticada ya no puede sacar la publicación de la carpeta elegida por el receptor.
- Añade una prueba de regresión en Windows x64 para la ruta señalada. Se conservan los nombres Unicode normales y el flujo de transferencia verificada.

## [0.22.0](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.22.0) — 2026-10-05

- Organiza el panel Omarchy y la app Windows de bandeja alrededor de las tareas habituales: recibir o controlar, encontrar y ubicar equipos, copiar archivos y ajustar permisos. La ayuda opcional empieza plegada y los equipos emparejados de Windows aparecen junto al mapa.
- Permite copiar un archivo entre Omarchy y Windows y pegarlo en el otro equipo. Los avisos de entrada aparecen incluso en otro workspace de Omarchy o escritorio virtual de Windows. El archivo se autentica y verifica antes de Pegar.
- Añade tres modos de aprobación para archivos copiados y enviados manualmente: **Preguntar siempre**, **Aceptar automáticamente** y **Preguntar y aceptar por un tiempo**. Muestra confirmación al guardar y cuenta regresiva; al vencer, ambas interfaces vuelven visiblemente a **Preguntar siempre**.
- Restaura en Windows el último modo de control, dirección y borde al iniciar. El arranque con Windows es opcional. Las compilaciones de prueba ofrecen un solo artefacto con los ejecutables y sus SHA; el release etiquetado ofrece un ZIP de Windows y su SHA256 contiguo.
- Pasaron pruebas físicas de archivos copiados en ambos sentidos entre Omarchy y Windows, incluidos nombres Unicode, duplicados, aprobación, rechazo y pegado. El [registro de pruebas](docs/TEST-RESULTS.es.md) distingue los casos que aún precisan más equipos o fallos simulados.

## [0.21.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.21.1) — 2026-10-03

- Empaqueta la app y el agente Windows x64 en un único ZIP con su archivo SHA-256 contiguo. La instalación y la actualización requieren una sola descarga.

## [0.21.0](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.21.0) — 2026-10-03

- Añade una aplicación Windows x64 de bandeja sobre el agente de consola existente, con pantallas en inglés y español, emparejamiento, descubrimiento, archivos y mapa arrastrable. Inserta el emblema de bandeja en el ejecutable gráfico y sitúa los equipos emparejados justo encima del mapa, con una guía para arrastrarlos.
- Compila y prueba ambos ejecutables Windows x64 en el workflow. Pruebas físicas con Windows 11 x64 verificaron emparejamiento, descubrimiento automático desde Omarchy, control en ambos sentidos, regreso por el borde y Escape, clics, las combinaciones de teclas probadas y una transferencia Omarchy→Windows.
- Sincroniza el portapapeles de texto en ambos sentidos entre Omarchy y Windows; el usuario confirmó ambos sentidos con equipos físicos.
- Renueva la vista previa y las guías bilingües para mostrar ratón, teclado, portapapeles, archivos y el espacio compartido entre Omarchy y Windows.
- Aclara en el panel el emparejamiento manual de Windows y el conflicto de identidades cuando dos sistemas reciben la misma IP.
- Publica la app y el agente Windows x64 del tag junto a sus SHA-256 en GitHub Releases; las guías ya apuntan a esas descargas.

## 0.20.0 — 2026-10-02

- Integra las mejoras de `alpha` en la rama publicada: el panel queda organizado en **Principal** y **Más**, con la preparación y el firewall en **Principal**, y explicaciones desplegables accesibles con el teclado.
- Muestra cuándo el receptor de archivos está escuchando y ofrece una acción específica para autorizar su puerto LAN. Corrige el manejo de la sesión de archivos tras una oferta y añade una prueba local del estado de escucha.
- Incluye la imagen promocional y las guías actualizadas en inglés y español. Se verificó una transferencia física en un sentido; la acción específica para el puerto de archivos y los demás casos requieren más pruebas físicas.

## 0.19.10 — 2026-10-02

- Destaca en los dos README y en la ficha del mercado las pruebas verificadas de ratón, teclado y regreso entre dos equipos.
- Corrige el registro de transferencia física: se verificó un sentido en `alpha`; quedan pendientes los demás casos. El código del agente y del widget no cambia.

## 0.19.9 — 2026-10-02

- Los selectores de archivo y carpeta se abren mediante el portal del escritorio en un proceso separado del agente. GTK/GVfs ya no se ejecuta dentro de Quickshell al abrir un selector. Se confirmó la apertura y cancelación del selector externo en el Omarchy afectado sin caída del shell.
- Se valida que la ruta seleccionada sea local antes de mostrarla en el panel. Los campos manuales siguen disponibles si falla el portal.

## 0.19.8 — 2026-10-02

- Se probaron los diálogos propios de Qt Quick para elegir archivos y carpetas. El panel real siguió fallando al pulsar **Elegir**; 0.19.9 mueve el selector fuera de Quickshell.

## 0.19.7 — 2026-10-02

- Se cierran los vínculos autenticados de malla al revocar la identidad de un equipo, aunque ya no controle la entrada. Se descartan mensajes pendientes del vínculo revocado y se libera la captura si ese equipo tenía el control.
- Se añadió una prueba de regresión local con conexiones cifradas de loopback. Sigue pendiente repetir la prueba de malla con varios equipos físicos.

## 0.19.6 — 2026-10-02

- Se publica un archivo del código fuente de cada tag y su suma SHA256 como assets del release.
- Se automatizan la generación y verificación de esos assets para las próximas versiones. No cambian el agente ni la instalación.

## 0.19.5 — 2026-10-02

- Se conservan los archivos personalizados de los servicios de usuario de SeamlessControl al instalar y retirar el agente.
- Se enumeran las dependencias de preparación directamente en el README para la revisión del marketplace.

## 0.19.4 — 2026-10-02

- Se corrigió el fallo intermitente de conexión al leer el mapa de teclado de Wayland desde una posición de archivo compartida en el receptor.
- Se mejoró el error del agente si todavía no puede leer el mapa del compositor. No cambió el emparejamiento ni el protocolo de red.

## 0.19.3 — 2026-10-02

- Se muestra en el panel el último error del agente durante la reconexión.
- Se aclaran los pasos para una reconexión transitoria y se documenta una interrupción observada al recargar el plugin con una sesión activa. No cambió el agente ni el protocolo de red.

## 0.19.2 — 2026-10-02

- Se añadieron documentos bilingües de contribución, ayuda, seguridad, conducta y versiones.
- Se añadieron plantillas de issues y pull requests y la configuración de actualización de dependencias.
- Se mejoraron el About, los temas y el canal privado de seguridad del repositorio. No cambió el protocolo del agente ni la instalación.

## [0.19.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.19.1) — 2026-10-02

- Se añadió el crédito del creador al panel.
- Se publicó un release solo de código fuente vinculado a un SHA exacto validado por CI.
- Se activó GitHub Sponsors y se documentó el proceso de release y del SHA del marketplace.

## 0.19.0 — 2026-10-02

- Se actualizó el estado de modificadores del teclado virtual para que atajos como Super+V lleguen al Omarchy receptor. Super+V y el regreso del puntero se confirmaron con dos equipos físicos.
- Se añadió una compilación bloqueada con Cargo y una guía bilingüe de publicación.

La historia anterior de desarrollo está en el [registro de commits](https://github.com/PuroDelphi/seamlesscontrol/commits/main/).
