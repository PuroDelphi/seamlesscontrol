# Portapapeles de imágenes y arrastre · validación alpha

Este documento registra las pruebas físicas pendientes antes de considerar estos cambios para `main`.

## Implementado

- [x] Mensajes PNG acotados en el canal de control autenticado, incluida la malla y la prevención de ecos.
- [x] Lectura de SVG y de formatos ráster admitidos en Wayland y de formatos registrados o mapas DIB/DIBV5 compatibles en Windows; conversión de los formatos no PNG a PNG acotado. Publicación de PNG en Omarchy y PNG más DIBV5 en Windows.
- [x] El permiso **Portapapeles** y la sesión de control activa rigen tanto el texto como las imágenes. Se ignoran las imágenes incompatibles o demasiado grandes sin interrumpir ratón y teclado.
- [x] Zona para soltar archivos y carpetas locales en la página Archivos del emisor, tanto en Omarchy como en Windows. Reutiliza la oferta cifrada, el permiso **Archivos**, la aprobación del receptor, el límite de tamaño, los hashes y la carpeta temporal. Nunca mueve ni borra el original.
- [x] Guías de usuario actualizadas en inglés y español.

## Pruebas físicas pendientes

- [ ] Omarchy → Windows: copiar una captura/imagen desde una aplicación y pegar sus píxeles en un editor de imágenes o chat. Comprobar que el texto sigue sincronizándose después.
- [ ] Windows → Omarchy: probar una imagen de Recortes y copias JPEG, GIF, WebP y SVG del navegador. Pegar sus píxeles y comprobar transparencia si la hay.
- [ ] Con el permiso Portapapeles desactivado, comprobar que no llegan texto ni imágenes. Sin conexión de control, comprobar que tampoco llegan.
- [ ] Soltar un archivo y una carpeta en la página Archivos del emisor en ambos sentidos. Comprobar oferta única, modos de aprobación, límite, rechazo, fin de sesión y conservación del original.
- [ ] En Windows, comprobar que todavía se puede arrastrar un equipo emparejado al mapa de pantallas después de habilitar la recepción de archivos arrastrados.
- [ ] Probar una imagen convertida mayor que 32 MiB y un arrastre no admitido. El control debe responder y la interfaz debe indicar cómo enviar el contenido como archivo.

## Puente nativo de arrastre aparte

Ya se puede soltar en la aplicación del emisor. Arrastrar continuamente desde el explorador de un sistema operativo a través del borde hasta el explorador del otro necesita un puente nativo de objetos de arrastre. Wayland envía ofertas a una superficie receptora y Windows utiliza OLE `IDataObject`/`IDropTarget`; la captura EIS/Windows actual solo transporta entrada. Antes de implementar ese puente hay que conservar permiso Archivos, aprobación, límites, rechazo de enlaces, cancelación y semántica de copia. No se debe presentar el arrastre directo entre escritorios como función disponible hasta terminarlo.
