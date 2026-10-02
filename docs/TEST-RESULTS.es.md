# Resultados actuales de las pruebas

[Guía de usuario](../README.es.md) · [Guía técnica](TECHNICAL.es.md) · [English](TEST-RESULTS.md) · [Registro detallado](FEASIBILITY.md) · [Casos pendientes](TESTING.md)

Dos equipos Omarchy físicos en la misma LAN se emparejaron tras mostrar y aprobar el mismo código de seis cifras en ambos. La conexión agotaba el tiempo al principio porque el firewall del receptor bloqueaba el puerto TCP elegido; una regla limitada a su interfaz local, subred, dirección y puerto permitió el emparejamiento. Después, el usuario confirmó que **Preparar regla LAN** y **Autorizar esta regla** funcionaron en el panel receptor.

El ratón cruzó desde el borde derecho del origen al receptor y regresó por el borde izquierdo del receptor en dos pruebas físicas separadas. El usuario confirmó que volvió a controlar el ratón local tras cada regreso. Escape físico también devolvió el control. Estas pruebas usaron los mismos dos equipos y una disposición concreta de monitores; el registro de desarrollo documenta las revisiones y fallos anteriores.

Con ambos agentes actualizados a la versión 0.19.0, **Super+V** en el teclado físico del origen abrió el historial del portapapeles en el receptor después de cruzar por el borde izquierdo. El usuario también confirmó que el puntero volvió al origen. Esto verifica los modificadores y el regreso en esa sesión física; no demuestra que funcionen todos los atajos o mapas de teclado.

En el episodio original de puntero atrapado había un proceso `connect` del origen suspendido en estado `T`: no podía procesar Escape, la devolución remota ni las señales de cierre. Además, se observó una recaptura inmediata después de Escape; el agente actual exige alejar el puntero 96 píxeles hacia el interior antes de rearmar el borde. También interpreta `solitaryBlockedBy: null` de Hyprland como desbloqueado cuando existe un espacio de trabajo activo.

Las pruebas locales de loopback e integración cubren emparejamiento, Noise XX, descubrimiento, cambios de IP simulados, oferta y aceptación de archivos, mensajes de retorno, entrada virtual y captura. La [guía técnica](TECHNICAL.es.md) enumera los comandos. El asistente de firewall tiene pruebas del alcance de la regla y la confirmación; los scripts de instalación tienen pruebas de conservación de identidad y retirada solo de paquetes registrados.

Siguen pendientes de verificación física: otros atajos y mapas de teclado, rueda y arrastre entre los dos equipos; archivos; sincronización del portapapeles; malla de varios receptores; otros bordes y disposiciones de monitores; bloqueo, suspensión y pérdida de red. La compatibilidad con Windows corresponde a una fase posterior.
