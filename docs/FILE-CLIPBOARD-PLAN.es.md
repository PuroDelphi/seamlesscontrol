# Plan: copiar un archivo aquí y pegarlo en otro equipo

Estado: flujo entre Omarchy y Windows implementado y verificado físicamente en `alpha` (2026-10-03); pendientes las pruebas ampliadas con más equipos y casos límite.

Decisión de implementación: la `Offer` de archivos existente, versionada y autenticada, sirve como evento. Se mantiene separada del canal de texto para que los agentes antiguos conserven su comportamiento y no se envíe contenido antes de aprobar.

Avance: [x] detección de copia y validación del origen (implementado y probado con tests unitarios); [x] servicio receptor y aprobación fuera del panel; [x] carpeta temporal verificada y pegado local; [x] CI Linux y Windows x64; [x] receptor automático Omarchy en TCP `47834` y observador del portapapeles; [x] navegación Inicio, Equipos, Archivos y Ajustes con acciones de archivo entrante encima de todas las secciones; [x] aceptación y pegado físicos Windows → Omarchy y Omarchy → Windows; [x] aviso en otro workspace de Omarchy y en otro escritorio virtual de Windows; [x] rechazo sin transferencia; [x] detección de Ctrl+C en ambos exploradores; [x] guías bilingües, capturas actuales de Archivos y Ajustes de Windows con datos ficticios y resumen de pruebas; [ ] prueba física Omarchy ↔ Omarchy y Windows ↔ Windows cuando estén disponibles esos equipos; [ ] casos pendientes del paso 5. Conservar este plan hasta terminar las comprobaciones.

## Experiencia de uso

1. Copiar un archivo normal en el explorador de un Omarchy o Windows emparejado. SeamlessControl muestra nombre y tamaño al equipo de destino elegido, sin transferir todavía el contenido. Si hay varios equipos, se elige el destino en la aplicación.
2. En el destino, pulsar **Recibir archivo copiado** en una notificación o en el panel. La oferta muestra emisor, nombre, tamaño y carpeta temporal. El usuario puede rechazarla.
3. Tras aprobarla, el protocolo cifrado actual descarga el archivo en una carpeta temporal privada, verifica tamaño y SHA-256 y nunca sobrescribe otro archivo. SeamlessControl coloca entonces una referencia al archivo local verificado en el portapapeles del destino. El usuario pulsa Pegar en el explorador de archivos para dejar una copia en la carpeta que desee.
4. Ambas interfaces muestran progreso, finalización, rechazo y errores. **Enviar archivo** seguirá disponible.

## Trabajo técnico

1. Utilizar la `Offer` de archivos existente, versionada y autenticada, para los archivos copiados en el canal independiente de ofertas. Conservar el canal del portapapeles de texto y evitar ecos. La oferta llevará un identificador aleatorio, identidad del equipo, nombre seguro, tamaño y caducidad; no expondrá una ruta local utilizable en el otro equipo. Los agentes antiguos deben conservar la sincronización de texto.
2. Observar los formatos del explorador: `CF_HDROP` en Windows y los tipos MIME del portapapeles Wayland que ofrezca el explorador activo, empezando por `text/uri-list`. Validar un archivo local normal. La primera versión rechazará URI remotas o ajenas a archivos, enlaces simbólicos, directorios y archivos virtuales no compatibles. Probar los exploradores reales de Omarchy antes de añadir otros formatos MIME.
3. Añadir aprobación y elección de equipo a ambas interfaces y un servicio de ofertas en segundo plano para que el receptor no tenga que pulsar antes **Esperar un archivo**. Explicar la regla del firewall LAN para el puerto independiente de archivos. Reutilizar identidades emparejadas, límites de tamaño por equipo y la transferencia Noise existente. Copiar no iniciará por sí solo la transferencia. Cancelar la oferta si cambia el portapapeles o el archivo de origen, se desconecta el equipo o caduca la oferta.
4. Guardar el archivo aprobado en una carpeta temporal privada con cuota. Publicar la referencia local en el portapapeles solo tras verificarlo: `CF_HDROP` en Windows y el formato URI que admita el explorador Omarchy. Conservar el archivo mientras el portapapeles lo necesite y definir la limpieza y recuperación cuando el usuario nunca lo pegue.
5. Probar Omarchy ↔ Omarchy, Omarchy ↔ Windows y Windows ↔ Windows en ambos sentidos: nombres Unicode y duplicados, cancelación, corte de red, archivo de origen cambiado, exceso de tamaño, disco lleno, sustitución del portapapeles y regresiones de texto.

## Alcance de la primera versión

El primer resultado puede ofrecer **Copiar → aprobar → Pegar**. Detectar que el usuario quiere pegar en cualquier carpeta antes de aprobar depende del explorador; Windows y Wayland usan formatos distintos. El panel de aceptación será el disparador seguro y, tras la verificación, Pegar en el explorador hará la colocación final. Estudiaremos objetos de portapapeles diferidos en Windows y ofertas de datos Wayland más adelante si las pruebas reales lo justifican.

Referencias: [formatos del portapapeles de Windows Shell](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard), [protocolo Wayland data-control](https://wayland.app/protocols/ext-data-control-v1), [lista de URI RFC 2483](https://www.rfc-editor.org/rfc/rfc2483.html).
