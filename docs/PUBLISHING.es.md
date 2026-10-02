# Publicar SeamlessControl

[Volver al README](../README.es.md) · [English](PUBLISHING.md)

El mercado de plugins de Omarchy revisa un **SHA completo de un commit de Git**. SeamlessControl compila el agente desde ese checkout del plugin; su instalador no descarga un ejecutable desde GitHub Releases. Usa `cargo build --locked`, y `agent/Cargo.lock` fija las versiones y sumas de comprobación de los crates. El [problema de descarga mutable de Omarchy Automations](https://github.com/omacom/omarchy-plugin-marketplace/issues/9276) se refería a un archivo de ejecución separado cuyo SHA256 esperado también se descargaba de un release mutable. Esa ruta de descarga no existe aquí. El instalador y sus solicitudes de privilegios igualmente necesitan la revisión del mercado.

1. Termina los cambios de interfaz, agente, manifiesto, documentación y lockfile; crea el commit y súbelo a `main`.
2. Espera a que pase el flujo **Validate** de GitHub Actions. El resumen **Marketplace submission** muestra automáticamente el SHA exacto de 40 caracteres que debes entregar, junto con el SHA256 del lockfile. No hay que editar ningún SHA a mano en cada publicación.
3. Crea el tag de versión y el GitHub Release de código fuente en ese mismo commit. El tag facilita encontrar la versión, pero el SHA completo sigue siendo el identificador de la revisión: un tag o nombre de release no verifica por sí solo la integridad de un archivo descargado.
4. Entrega el SHA completo del commit y mantén el código revisado en ese commit mientras lo validan. Cualquier commit posterior inicia un nuevo flujo con un SHA nuevo.

Para comprobarlo localmente antes de entregarlo, ejecuta `bash scripts/submission-pin.sh`. Rechaza cambios sin confirmar o un commit local distinto del `main` publicado, valida el plugin, compila con el lockfile e imprime el mismo SHA completo del commit y el SHA256 del lockfile.

Los comandos de usuario `omarchy plugin add` y `omarchy plugin update` siguen obteniendo el `HEAD` actual; la validación por SHA del mercado no fija esos comandos a un commit. Consulta la [política de verificación](https://github.com/omacom/omarchy-plugin-marketplace/blob/main/VERIFICATION.md).

El GitHub Release v0.19.1 no incluye un agente precompilado. Si más adelante se ofrece uno, el checkout revisado deberá fijar su SHA256 exacto y el instalador deberá verificarlo antes de instalarlo; crear un tag de release por sí solo no resolvería una descarga mutable.
