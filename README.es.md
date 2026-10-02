# SeamlessControl para Omarchy

[![Validación](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml/badge.svg)](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml) · [Última versión](https://github.com/PuroDelphi/seamlesscontrol/releases/latest) · [Licencia MIT](LICENSE)

Usa un ratón y teclado entre equipos Omarchy cercanos. Cruza el borde de una pantalla para controlar el siguiente equipo y vuelve por el borde para regresar. La configuración probada es Omarchy con Omarchy; `alpha` incluye un receptor Windows x64 para pruebas entre plataformas.

**Idioma:** [English](README.md) · Español

**Uso del panel:** [Guía ilustrada](docs/USER-GUIDE.es.md)

**Windows x64 alpha:** [Agente Windows y primera prueba entre plataformas](docs/WINDOWS-ALPHA.es.md)

**Para desarrolladores:** [Guía técnica](docs/TECHNICAL.es.md) · [Resultados de pruebas](docs/TEST-RESULTS.es.md) · [Plan](PLAN.md)

## Probado con dos equipos Omarchy físicos

En una misma red local, los dos equipos se emparejaron con códigos de seis cifras coincidentes. El ratón cruzó al receptor y regresó por el borde opuesto dos veces; **Escape** también devolvió el control. El atajo físico **Super+V** abrió el historial del portapapeles en el receptor. Un archivo de texto se ofreció, aceptó y entregó después de autorizar el puerto de archivos independiente en el receptor.

Son resultados observados con una pareja de equipos. El [registro de pruebas](docs/TEST-RESULTS.es.md) detalla las versiones, las comprobaciones locales y los casos que aún requieren verificación física.

Las capturas del panel usan nombres de equipo y direcciones de red ficticios.

## Instalar en ambos equipos

Usa el comando estándar de Omarchy:

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
```

Abre **SeamlessControl** desde la barra de Omarchy. El panel comienza en inglés; pulsa **Español** arriba para cambiar el idioma. En **Preparar este equipo**, pulsa **Instalar agente**. Se abrirá una terminal de Omarchy: instalará los paquetes que falten, compilará el agente y pedirá autorización si es necesaria. Vuelve al panel cuando la terminal indique que terminó. Repite en el segundo equipo.

![Controles de idioma e instalación de SeamlessControl](docs/images/setup-es.png)

La captura muestra un agente ya instalado, por eso el botón dice **Actualizar agente**. En un equipo nuevo, dice **Instalar agente**.

La primera compilación descarga dependencias de Rust y puede tardar. La preparación instala los paquetes ausentes `rust`, `avahi` y `wl-clipboard` y puede pedir autorización del sistema. El panel detecta automáticamente el agente. El widget instalado por sí solo todavía no puede compartir la entrada.

## Actualizar en ambos equipos

Detén cualquier sesión activa de SeamlessControl. En **cada** Omarchy, actualiza el widget con el comando estándar:

```bash
omarchy plugin update seamlesscontrol.control
```

Después ejecuta `omarchy restart shell` y vuelve a abrir el panel para que Omarchy cargue la interfaz actualizada. Pulsa **Actualizar agente** en **Preparar este equipo**. La terminal recompilará y reemplazará el agente sin borrar las claves de emparejamiento ni el mapa de equipos. Cierra la terminal cuando termine y vuelve a iniciar la recepción o el uso compartido. Actualiza ambos equipos antes de usar una versión nueva del agente. Un cambio de idioma del widget no requiere actualizarlo.

## Conectar dos equipos Omarchy

**Emparejar** autoriza un equipo una vez; **Conectar** inicia la sesión de control cada vez que quieras usarlo. Ubicar los equipos en el mapa solo define el borde de cruce.

1. En el equipo **que vas a controlar**, pulsa **Recibir control**. Deja la dirección vacía para usar la IP de la red local y el puerto TCP `47832`. Espera a que indique **Disponible**.
2. En el equipo con el ratón físico, busca el receptor en **Equipos en la red** y pulsa **Emparejar**. Si no aparece, pulsa **Buscar** o escribe su `IP:puerto` en el campo manual.
3. **Ambos paneles** muestran un código de seis cifras. Compáralos y pulsa **Coincide · aprobar aquí** en **los dos equipos**. El código se genera automáticamente; no se puede escribir ni cambiar. La **Identidad local**, más larga, es la huella del equipo, no el código. Si el receptor sigue en **Disponible** y no aparece el código, revisa la red y el firewall.
4. En **Mapa de equipos**, coloca el otro equipo junto a **Este equipo** en **ambos equipos**, tal como están físicamente. Por ejemplo, si el receptor está a la derecha del equipo con ratón, colócalo a la derecha en el origen; en el receptor, coloca el origen a la izquierda. Así podrás regresar cruzando el borde izquierdo del receptor. Puedes arrastrar las fichas o usar Tab, Enter y las flechas. El mapa solo guarda la dirección del cruce.
5. En el equipo con ratón, pulsa **Conectar** junto al receptor descubierto. Espera a **Listo** y cruza el **borde exterior indicado de todos los monitores del origen**. Para volver, cruza el borde del receptor hacia el origen o pulsa **Escape** en el teclado físico.

![Emparejar y Conectar son acciones distintas en el panel](docs/images/connect-context-es.png)

![La acción Conectar junto a un receptor emparejado](docs/images/connect-button-es.png)

![El panel de origen indica Listo cuando puede capturar el cruce de borde](docs/images/session-es.png)

Si el puntero no vuelve, pulsa **Devolver control al origen** en el receptor. **Cortar entrada remota · emergencia** desconecta y pausa la recepción; después pulsa **Reanudar recepción**. Para detener una sesión iniciada desde el panel, pulsa **Terminar sesión iniciada desde el panel**.

Si el origen permanece en **Preparando la captura del ratón y teclado** durante 15 segundos, el panel ofrece **Reiniciar captura de este equipo**. Cierra ese intento y reinicia el servicio de captura del escritorio; puede interrumpir otras aplicaciones que comparten pantalla. Después pulsa **Conectar** de nuevo. La [guía ilustrada](docs/USER-GUIDE.es.md) muestra todo el proceso en el panel, soluciones según el estado, transferencia de archivos y para qué sirve la malla experimental de varios equipos.

## Si el emparejamiento agota el tiempo

El cambio de firewall se hace en el **receptor**. En **Firewall · solo en el receptor**, escribe el mismo puerto TCP de **Recibir control**, pulsa **Preparar regla LAN** y revisa la interfaz, subred y destino que se muestran. Pulsa **Autorizar esta regla** para aprobarla en Omarchy. La regla se limita a la red local detectada; el panel no la abre a Internet.

![Vista previa de la regla LAN en el panel receptor](docs/images/firewall-es.png)

Si eliges otro puerto de recepción, úsalo también en la sección de firewall. **Antes del primer envío**, autoriza el puerto de archivos separado (`47833` por defecto) en el receptor con **Preparar regla LAN para archivos**, en **Archivos**. El puerto de control (`47832` por defecto) no abre la transferencia de archivos. Pulsa **Esperar un archivo** otra vez para cada archivo.

## Desinstalar

Detén cualquier sesión activa. En **Preparar este equipo**, pulsa **Retirar agente** y confirma. La terminal retirará el agente y solo los paquetes que SeamlessControl haya registrado como instalados por él. Conservará los archivos de servicio modificados, las claves de emparejamiento y la posición de los equipos para una posible reinstalación. Después retira el widget con el comando estándar de Omarchy:

```bash
omarchy plugin remove seamlesscontrol.control
```

Si autorizaste una regla de firewall, retírala **antes** de quitar el widget en el receptor: `bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh remove <PUERTO>`. El script pide confirmación y autorización del sistema.

## Más información

- [Ayuda y solución de problemas](SUPPORT.es.md)
- [Contribuir](CONTRIBUTING.es.md) · [Normas de conducta](CODE_OF_CONDUCT.es.md) · [Política de seguridad](SECURITY.es.md)
- [Notas de versiones](CHANGELOG.es.md) · [Último release de código fuente](https://github.com/PuroDelphi/seamlesscontrol/releases/latest)
- [Guía técnica: comandos manuales, seguridad, servicios y pruebas](docs/TECHNICAL.es.md)
- [Guía ilustrada del panel y sus funciones](docs/USER-GUIDE.es.md)
- [Límites actuales y resultados de pruebas físicas](docs/TEST-RESULTS.es.md)
- [Plan](PLAN.md)
- [Publicación y SHA exacto para el mercado de plugins](docs/PUBLISHING.es.md)

Licencia MIT. Consulta [LICENSE](LICENSE).

## Apoya este proyecto

Si SeamlessControl te resulta útil, puedes apoyar su mantenimiento mediante [GitHub Sponsors](https://github.com/sponsors/PuroDelphi) o [PayPal](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ). Las donaciones son opcionales y no cambian la licencia MIT.

[![Escanea para donar por PayPal](docs/images/paypal-qr.png)](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ)
