# Publicar SeamlessControl

[Volver al README](../README.es.md) · [English](PUBLISHING.md)

El mercado de plugins de Omarchy valida un **SHA completo de un commit de Git**. SeamlessControl compila el agente desde el mismo checkout del plugin, por lo que no descarga un archivo de ejecución separado que haya que fijar. El instalador usa `cargo build --locked`: `agent/Cargo.lock` fija las versiones y sumas de comprobación de las dependencias de crates.io. Esto cumple el propósito del pin de un archivo externo para esta instalación desde el código fuente.

1. Termina los cambios de interfaz, agente, manifiesto, documentación y lockfile; crea el commit y súbelo a `main`.
2. Espera a que pase el flujo **Validate** de GitHub Actions. El resumen **Marketplace submission** muestra automáticamente el SHA exacto de 40 caracteres que debes entregar, junto con el SHA256 del lockfile. No hay que editar ningún SHA a mano en cada publicación.
3. Entrega ese SHA de commit y mantén `main` en ese commit mientras lo validan. Cualquier commit posterior inicia un nuevo flujo con un SHA nuevo.

Para comprobarlo localmente antes de entregarlo, ejecuta `bash scripts/submission-pin.sh`. Rechaza cambios sin confirmar o un commit local distinto del `main` publicado, valida el plugin, compila con el lockfile e imprime el mismo SHA completo del commit y el SHA256 del lockfile.

Los comandos de usuario `omarchy plugin add` y `omarchy plugin update` siguen obteniendo el `HEAD` actual; la validación por SHA del mercado no fija esos comandos a un commit. Consulta la [política de verificación](https://github.com/omacom/omarchy-plugin-marketplace/blob/main/VERIFICATION.md).

La instalación normal no usa un archivo de GitHub Releases. Si más adelante se ofrece un agente precompilado, habrá que fijar y comprobar por separado la suma de ese archivo antes de instalarlo.
