# Notas de versiones

[English](CHANGELOG.md) · [Todos los releases de GitHub](https://github.com/PuroDelphi/seamlesscontrol/releases)

Este archivo resume los cambios publicados. Consulta los [resultados de pruebas](docs/TEST-RESULTS.es.md) para conocer las verificaciones y los casos que faltan.

## Sin publicar · alpha

- Añade un receptor de consola para Windows x64 compatible con el emparejamiento y el protocolo cifrado de Omarchy. Puede solicitar el regreso por el borde de entrada y enviar o recibir archivos aprobados.
- Compila y prueba el ejecutable Windows x64 en un workflow de `alpha`. En una prueba física con Windows 11 x64 se verificaron emparejamiento, cruce desde Omarchy, regreso por el borde, clic, una tecla, Super+E y regreso con Escape.
- Aclara en el panel el emparejamiento manual de Windows y el conflicto de identidades cuando dos sistemas reciben la misma IP.

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
