# Notas de versiones

[English](CHANGELOG.md) · [Todos los releases de GitHub](https://github.com/PuroDelphi/seamlesscontrol/releases)

Este archivo resume los cambios publicados visibles para los usuarios. El proyecto es experimental; consulta los [resultados de pruebas](docs/TEST-RESULTS.es.md) para saber qué se ha verificado.

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
