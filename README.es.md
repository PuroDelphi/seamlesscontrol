# SeamlessControl

[![Validación](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml/badge.svg)](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml) · [Descargar Windows x64](https://github.com/PuroDelphi/seamlesscontrol/releases/latest) · [English](README.md) · [MIT](LICENSE)

**Un ratón y un teclado para tus equipos Omarchy y Windows en una LAN privada.** Cruza el borde exterior del escritorio para controlar otro equipo; vuelve por el borde de entrada o pulsa **Escape en el teclado físico del origen**.

También puedes compartir texto durante la conexión y copiar **un archivo local cada vez** entre equipos emparejados. El receptor decide cómo aprobar los archivos; solo quedan disponibles después de verificarse.

## Elige por dónde empezar

| Quiero… | Dónde está explicado |
|---|---|
| Instalar el plugin y conectar desde Omarchy | [Guía ilustrada Omarchy](docs/USER-GUIDE.es.md) |
| Instalar la app, recibir o controlar desde Windows | [Guía ilustrada Windows](docs/WINDOWS.es.md) |
| Copiar y pegar archivos o enviarlos a una carpeta | [Guía Omarchy](docs/USER-GUIDE.es.md) / [Guía Windows](docs/WINDOWS.es.md), sección Archivos |
| Resolver un problema de conexión | Sección de problemas de cada guía; [pedir ayuda](SUPPORT.es.md) |
| Usar comandos o conocer permisos y límites | [Guía técnica](docs/TECHNICAL.es.md) |

**Origen** es el equipo donde están el ratón y el teclado físicos que vas a usar. **Receptor** es el equipo que quieres controlar. Los papeles pueden cambiar para otra sesión; no necesitas emparejar de nuevo por invertirlos.

## 1. Instala en ambos equipos

### Omarchy

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
```

Abre **SeamlessControl** desde la barra y pulsa **Inicio → Preparar este equipo → Instalar agente**. Se abre una terminal para instalar las dependencias y compilar el agente; autoriza la instalación cuando el sistema lo pida. Espera a que termine antes de conectar. El idioma inicial es inglés; selecciona **Español** arriba.

![Inicio del plugin Omarchy: preparación del agente y recepción de control](docs/images/omarchy-overview-es.png)

### Windows x64

1. En la [última versión](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), descarga **`seamlesscontrol-windows-x64.zip`** desde **Assets**.
2. **Extrae el ZIP** en una carpeta propia. Mantén juntos `seamlesscontrol.exe` y `seamlesscontrold.exe`; no los ejecutes dentro del ZIP.
3. Abre **`seamlesscontrol.exe`**. En el primer inicio recibe control por TCP `47832`. Después restaura el último modo que dejaste activo, incluso una conexión saliente.
4. Si Windows pregunta por el acceso a la red, permite solo **Redes privadas**. Si falta WebView2, instala [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

Cerrar la ventana **no** cierra la aplicación: queda en la bandeja. Para salir, usa **Exit SeamlessControl** en su menú. El arranque automático se activa opcionalmente en **Ajustes → Iniciar con Windows**.

La [guía Windows](docs/WINDOWS.es.md) incluye verificación SHA-256, firewall y descarga de compilaciones **alpha**. No confundas una compilación de prueba de Actions con la última versión publicada.

## 2. Empareja, ubica y conecta

1. **En el receptor:** activa **Recibir control**. En Omarchy, espera a **Disponible**; en Windows, comprueba que la recepción esté activa en Inicio.
2. **En el origen:** abre **Equipos**. En **1 · Emparejar por dirección**, escribe la `IP:puerto` privada del receptor; o usa **2 · Equipos cercanos → Emparejar**. Son alternativas, no dos pasos obligatorios.
3. **En ambos equipos:** compara el código de **seis cifras** y aprueba solo si coincide. La huella de identidad larga no es ese código. El receptor aparecerá en **3 · Equipos emparejados**.
4. **En el origen:** coloca el receptor en **4 · Mapa de pantallas**, junto a tu equipo y en el lado donde esté físicamente su pantalla. Esto guarda el borde: **todavía no conecta**.
5. **Inicia la sesión:** en Omarchy, pulsa **Conectar** junto al receptor descubierto, o usa **Ajustes → Conectar por IP** si no aparece. En Windows, **Conectar** en la fila prepara Inicio; pulsa **Conectar otra vez en Inicio** para arrancar la sesión.
6. Cuando esté listo, cruza el **borde exterior del escritorio completo**, no la separación entre monitores del mismo equipo. Vuelve por el borde de entrada del receptor o con **Escape** físico. El regreso deja la sesión preparada para otro cruce; detenerla es una acción distinta.

![Equipos de Windows: alternativas de emparejamiento, pares y ubicación en el mapa](docs/images/windows-devices-es.png)

Las imágenes muestran interfaces con datos ficticios de documentación; la guía de cada plataforma explica cómo se obtuvieron.

**Si Omarchy ya recibe y quieres iniciar allí el emparejamiento**, usa **Detener recepción para emparejar** junto al paso 1. El corte de emergencia solo pausa: no libera el receptor para iniciar el emparejamiento. Windows detiene temporalmente su propia sesión al emparejar y restaura el modo anterior al terminar. En ambos casos, el receptor del **otro** equipo debe seguir activo.

## 3. Elige cómo compartir

| Función | Qué necesitas | Puerto en el receptor |
|---|---|---|
| Ratón, teclado y texto del portapapeles | Emparejar + Recibir control + Conectar | TCP `47832` por defecto |
| Copiar en un explorador y pegar en el otro | Apps abiertas + emparejamiento; aprobar la oferta si corresponde y esperar la verificación antes de Pegar | TCP `47834` |
| Enviar a una carpeta elegida | Emparejamiento + Archivos → Esperar un archivo en el receptor; elegir y enviar en el origen | TCP `47833` por defecto |
| Encontrar receptores cercanos | Recepción activa y descubrimiento permitido en la LAN | UDP `5353` (mDNS) |

**Los dos flujos de archivos no requieren una sesión de control.** **Esperar un archivo** no activa la copia y pegado: es el envío manual, de una oferta cada vez. Abrir el puerto de control tampoco abre los de archivos.

En **Ajustes → Aprobación de archivos entrantes**, puedes preguntar por cada oferta (predeterminado), aceptar automáticamente desde equipos emparejados o aprobar por un tiempo. El modo temporal pregunta por el primer archivo de cada equipo y vuelve a **Preguntar siempre** al vencer o reiniciar. Las [guías ilustradas](docs/USER-GUIDE.es.md) explican el límite de tamaño, la selección del destino y qué hacer si no llega una oferta.

No abras estos puertos a Internet ni configures reenvío en el router. Usa las acciones de firewall de la interfaz en el **receptor**, limitadas a la LAN privada.

## Actualizar o retirar

### Omarchy

Termina las sesiones en ambos equipos. Actualiza el plugin y recarga el shell:

```bash
omarchy plugin update seamlesscontrol.control --yes
omarchy restart shell
```

Abre el panel y pulsa **Actualizar agente**. Hazlo en cada Omarchy implicado antes de reanudar. Se conservan la identidad, los emparejamientos y el mapa.

Para retirarlo, usa **Ajustes → Retirar agente** y después `omarchy plugin remove seamlesscontrol.control`. La retirada conserva los datos de emparejamiento; las reglas de firewall se retiran por separado, como explica la [guía técnica](docs/TECHNICAL.es.md).

### Windows

Para actualizar, sal desde la bandeja, extrae el ZIP nuevo, reemplaza **ambos ejecutables** y abre la app otra vez. Se conservan los datos en `%LOCALAPPDATA%\SeamlessControl`.

Para desinstalar, primero desactiva **Ajustes → Iniciar con Windows**, sal desde la bandeja y borra la carpeta de los ejecutables. Retira las reglas `SeamlessControl TCP … Private LAN` y `SeamlessControl UDP 5353 Private LAN` que hayas autorizado. Borra la carpeta de datos **solo** si deseas una identidad nueva: tendrás que volver a emparejar.

## Alcance y documentación del proyecto

El [registro de pruebas](docs/TEST-RESULTS.es.md) recoge control, regreso, texto y copia de archivos en ambos sentidos entre Omarchy y Windows 11 x64 físicos. No equivale a una garantía para todas las combinaciones: la malla de varios equipos es experimental, y otras disposiciones, suspensión y fallos de red tienen escenarios pendientes. El bloqueo manual durante control remoto en Omarchy aún requiere verificación física; devuelve y detén el control antes de bloquear manualmente.

Las guías anteriores describen el uso actual. Las notas `docs/RELEASE-*.md`, el changelog y el registro de viabilidad conservan **historia de versiones**, no instrucciones actuales de instalación.

[Ayuda](SUPPORT.es.md) · [Contribuir](CONTRIBUTING.es.md) · [Seguridad](SECURITY.es.md) · [Novedades](CHANGELOG.es.md) · [Normas de conducta](CODE_OF_CONDUCT.es.md)

Powered by JhonnySuarez - PuroDelphi. Apoya el proyecto mediante [GitHub Sponsors](https://github.com/sponsors/PuroDelphi) o [PayPal](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ).

[![QR para donar por PayPal](docs/images/paypal-qr.png)](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ)
