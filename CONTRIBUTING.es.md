# Contribuir a SeamlessControl

[English](CONTRIBUTING.md) · [Normas de conducta](CODE_OF_CONDUCT.es.md)

Se aceptan correcciones, documentación, traducciones y funciones concretas. Los [resultados de pruebas](docs/TEST-RESULTS.es.md) distinguen las verificaciones físicas de las simuladas. Describe las funciones nuevas conforme a la evidencia disponible.

## Antes de empezar

1. Busca [issues existentes](https://github.com/PuroDelphi/seamlesscontrol/issues) y consulta los [casos pendientes de verificación](docs/TESTING.md). Abre un issue para un cambio grande, del protocolo o de plataforma para conversar primero sobre el comportamiento esperado. Una corrección pequeña puede ir directamente a un pull request.
2. Lee la [guía técnica](docs/TECHNICAL.es.md) y mantén los procesos de usuario final dentro del panel. La instalación debe seguir usando `omarchy plugin add`; el panel administra las dependencias adicionales del agente.
3. Trabaja en una rama basada en el `main` actual. Separa los cambios independientes en pull requests distintos.

## Comprobaciones locales

```bash
omarchy plugin validate .
cargo fmt --manifest-path agent/Cargo.toml --check
cargo test --locked --manifest-path agent/Cargo.toml --all-targets
cargo clippy --locked --manifest-path agent/Cargo.toml --all-targets -- -D warnings
bash -n packaging/*.sh scripts/*.sh tests/*.sh
```

Algunas pruebas integradas necesitan una sesión Omarchy activa o un segundo equipo. Indica qué comprobaste, en qué entorno y qué falta verificar. Para cambios visuales del panel, incluye capturas en inglés y español. Actualiza ambas guías si cambia el comportamiento visible. Confirma `agent/Cargo.lock` cuando cambien dependencias.

## Mantener las imágenes de las guías

Desde la raíz del repositorio:

```bash
python3 scripts/render-omarchy-doc-screenshots.py
python3 scripts/render-windows-doc-screenshots.py
```

El primero requiere Quickshell y los componentes instalados en `/usr/share/omarchy/shell`; `--shell-source` permite elegir otro checkout del shell y `--output` una carpeta de revisión. Renderiza el QML real con un backend de datos ficticios, sin ejecutar el agente ni usar la configuración del escritorio. El segundo requiere Chromium y renderiza el HTML de la app Windows en Linux, sin un agente Windows. Ambos regeneran inglés y español y rotulan la procedencia.

Inspecciona el resultado en las guías renderizadas: texto legible, código de emparejamiento, mapa y botones sin recortar. No confundas estas imágenes con pruebas físicas del control, diálogos nativos o firewall. Retira las imágenes que ya no tengan referencias cuando cambies el recorrido.

## Pull request

Explica el problema, el cambio visible, las pruebas y cualquier permiso, puerto, paquete o dato persistente nuevo. No subas binarios, identidades locales, datos de emparejamiento, material SSH, direcciones IP privadas reales ni contenido de usuarios. El flujo CI debe pasar antes de una publicación. El mantenedor puede pedir ajustes o posponer una función que amplíe el acceso a entrada o red sin pruebas suficientes.

Informa de vulnerabilidades por la [vía privada](SECURITY.es.md), no mediante un issue o pull request público.
