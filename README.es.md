# SeamlessControl

[![Validación](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml/badge.svg)](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml) · [Última versión y descargas Windows x64](https://github.com/PuroDelphi/seamlesscontrol/releases/latest) · [MIT](LICENSE) · [English](README.md)

**Un ratón. Un teclado. Todas tus pantallas al alcance.** Cruza el borde para trabajar en el siguiente equipo y vuelve cruzando en sentido contrario o pulsando **Escape**. SeamlessControl une Omarchy y Windows en tu red privada con el mismo protocolo de emparejamiento y control.

![Pantallas Omarchy y Windows unidas por ratón, teclado, portapapeles y archivos](preview.png)

![Equipos Omarchy y Windows en un mismo espacio de trabajo](docs/images/ecosystem.svg)

- **Muévete con naturalidad:** el puntero, los clics, la rueda y el teclado te siguen. Los atajos con Windows/Super funcionan en el destino.
- **Mantén el ritmo:** sincronización del portapapeles de texto, regreso por el borde o con Escape y recuperación de la entrada local si termina la conexión.
- **Mantén despierto el origen:** cuando un Omarchy controla otro equipo, su panel evita el bloqueo automático por inactividad hasta que regrese el control.
- **Encuentra equipos cercanos:** el descubrimiento LAN muestra receptores disponibles. Puedes introducir `IP:puerto` si la red bloquea el descubrimiento.
- **Empareja con seguridad:** compara el código de seis cifras en ambos equipos. Cada uno recuerda la identidad del otro y exige revisar cualquier cambio de clave.
- **Copia y pega archivos entre equipos:** copia un archivo en el explorador de Windows u Omarchy, aprueba la oferta visible en el receptor y pégalo en la carpeta de destino.
- **Elige cómo se autorizan los archivos:** pregunta siempre, acepta automáticamente desde equipos emparejados o aprueba durante los minutos que elijas. La opción temporal muestra una cuenta regresiva y vuelve a preguntar al vencer.
- **Usa una interfaz familiar:** un panel Omarchy organizado y una aplicación Windows que permanece en la bandeja. Ambas tienen inglés y español, confirman los ajustes guardados y muestran claramente los archivos entrantes.

Usa tus equipos Omarchy y Windows en un mismo espacio: el control cruza en ambos sentidos y el emparejamiento, el descubrimiento y el portapapeles de texto facilitan cada sesión. Seguiremos mejorando SeamlessControl continuamente. La [guía técnica](docs/TECHNICAL.es.md) explica la arquitectura, los permisos y las verificaciones.

**Comprobado en equipos físicos Omarchy y Windows 11 x64:** control de ratón y teclado en ambos sentidos, regreso por el borde y con Escape, portapapeles de texto en ambos sentidos y copia y pegado de archivos aprobados en ambos sentidos. El aviso de archivo entrante apareció en otro workspace de Omarchy y en otro escritorio virtual de Windows; rechazar una oferta no transfirió ningún archivo. El [registro de pruebas](docs/TEST-RESULTS.es.md) distingue estos resultados de las comprobaciones pendientes con más equipos.

## Pantallas sencillas, decisiones claras

**Omarchy** deja la instalación y la sesión activa en **Inicio**. **Equipos** guía el emparejamiento en cuatro pasos: dirección, receptores cercanos, equipos emparejados y mapa. Si este equipo ya recibe control, **Detener recepción para emparejar** aparece junto al paso 1; el corte de emergencia solo pausa el receptor. **Archivos** permite copiar y pegar o enviar deliberadamente; **Ajustes** contiene la aprobación y el firewall. La ayuda opcional se abre cuando la necesitas.

![Inicio organizado del panel Omarchy](docs/images/omarchy-overview-es.png)

**Windows** tiene **Inicio, Equipos, Archivos y Ajustes**. Equipos sigue los mismos cuatro pasos y sitúa los emparejados justo encima del mapa arrastrable. Al iniciar un emparejamiento, la app detiene brevemente su propia sesión de control y restaura el modo anterior al terminar. También recuerda el último modo de control, dirección y borde. Puedes cerrar la ventana y dejarla disponible en la bandeja.

![Pasos de emparejamiento y mapa de pantallas en Windows](docs/images/windows-devices-es.png)

Para un archivo entrante de un equipo **emparejado**, elige cuánto quieres intervenir:

| Modo de aprobación | Qué ocurre |
|---|---|
| **Preguntar siempre** | Revisa remitente, archivo y tamaño en un solo aviso visible. |
| **Aceptar automáticamente** | Recibe archivos autenticados sin mostrar una solicitud. |
| **Preguntar y aceptar por un tiempo** | Aprueba el primer archivo y recibe los siguientes de ese equipo hasta que termine la cuenta regresiva. Después vuelve a **Preguntar siempre**. |

El archivo se verifica antes de quedar disponible para **Pegar**. El aviso llega al workspace activo de Omarchy o al escritorio virtual de Windows aunque el panel principal esté cerrado. [Consulta la guía ilustrada de archivos](docs/USER-GUIDE.es.md#elegir-cómo-se-aprueban-los-archivos-entrantes).

## Empezar en Omarchy

Instala el plugin en cada equipo Omarchy con el comando estándar:

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
```

Abre **SeamlessControl** desde la barra de Omarchy. En **Preparar este equipo**, pulsa **Instalar agente**. El panel instala los paquetes necesarios y el agente; solicitará autorización del sistema cuando corresponda. Empieza en inglés; puedes escoger **Español** arriba.

![Instalación del agente desde el panel Omarchy](docs/images/setup-es.png)

## Empezar en Windows x64

Abre la [última versión y sus notas de instalación](https://github.com/PuroDelphi/seamlesscontrol/releases/latest). En **Assets**, descarga el único **`seamlesscontrol-windows-x64.zip`** y extráelo en una carpeta propia. Ya contiene juntos los dos ejecutables necesarios. Haz doble clic en **`seamlesscontrol.exe`**. El archivo contiguo `seamlesscontrol-windows-x64.zip.sha256` permite verificar la descarga. La primera vez comienza a recibir en el puerto `47832`; después restaura el último modo de control, incluida una conexión saliente. Sigue activo en la bandeja al cerrar la ventana. Puedes escoger **Español** arriba a la derecha.

Para iniciarlo automáticamente, activa **Ajustes → Iniciar con Windows** en la app. Se abrirá en la bandeja cuando inicies sesión; puedes desactivarlo desde la misma pantalla.

![La aplicación SeamlessControl para Windows con la última conexión restaurada](docs/images/windows-home-es.png)

![Copiar un archivo en Windows y ofrecerlo a un equipo emparejado](docs/images/windows-files-es.png)

Windows 11 normalmente incluye WebView2. Si la aplicación indica que falta, instala [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) y ábrela de nuevo. Si Windows pregunta por el acceso a la red, permite solamente **Redes privadas**. En **Ajustes → Firewall de Windows**, la app puede pedir autorización de administrador para reglas LAN privadas del puerto de control, el de archivos y el autodescubrimiento mDNS (`UDP 5353`). Los botones TCP toman los puertos que ves en la interfaz.

[Guía Windows paso a paso](docs/WINDOWS.es.md) · [Guía ilustrada Omarchy](docs/USER-GUIDE.es.md)

## Conectar tus pantallas

1. En el equipo que quieres controlar, pulsa **Recibir control** y espera a **Disponible**. La app Windows lo inicia la primera vez y lo restaura si lo dejaste activo.
2. En el equipo con el ratón físico, abre **Equipos**. Usa **1 Emparejar por dirección** si conoces la `IP:puerto` privada del receptor o **2 Equipos cercanos** y pulsa **Emparejar**. Compara el código de seis cifras y aprueba en **ambos** equipos. El par aparecerá en **3 Equipos emparejados**. En Omarchy, si este equipo ya recibe control y quieres iniciar el emparejamiento desde aquí, pulsa **Detener recepción para emparejar** junto al paso 1; después podrás volver a activar Recibir control.
3. En **4 Mapa de pantallas**, coloca el receptor emparejado en el lado donde está su pantalla. Omarchy y Windows tienen un mapa visual. En Windows, los equipos emparejados aparecen justo encima del mapa: arrastra uno siguiendo la flecha, pulsa un lado o usa las flechas del teclado. Su posición rellena el borde al preparar **Conectar**.
4. Pulsa **Conectar** en el equipo con el ratón físico. Cuando indique **Listo**, cruza el **borde exterior** elegido. Regresa por el borde del receptor o pulsa **Escape** en el teclado físico.

**Emparejar** guarda la confianza una vez; **Conectar** inicia cada sesión. Mover una ficha del mapa solo define el lado de cruce. Para enviar archivos, abre **Archivos** en el receptor, autoriza si hace falta su puerto LAN independiente `47833`, pulsa **Esperar un archivo** y después elige y envía el archivo desde el origen. El receptor aprueba la oferta. Para copiar normalmente, permite TCP `47834` en el receptor, copia un archivo en el explorador Windows u Omarchy, aprueba el aviso de entrada y pégalo en la carpeta de destino. No hace falta una sesión de control activa.

![Emparejar y Conectar en el panel Omarchy](docs/images/connect-context-es.png)

## Actualizar y desinstalar

**Omarchy:** termina las sesiones y actualiza el plugin en cada equipo Omarchy:

```bash
omarchy plugin update seamlesscontrol.control --yes
```

`--yes` aplica la actualización sin abrir la vista larga de cambios ni pedir confirmación. Ejecuta `omarchy restart shell`, abre de nuevo SeamlessControl y pulsa **Actualizar agente** en **Preparar este equipo**. Se conservan las claves y el mapa.

**Windows:** sal de SeamlessControl desde el menú de la bandeja, descarga `seamlesscontrol-windows-x64.zip` de la [última versión](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), extráelo y reemplaza ambos ejecutables en su carpeta y abre `seamlesscontrol.exe` otra vez. Se conservan las claves en `%LOCALAPPDATA%\SeamlessControl`.

Para retirarlo de Omarchy, pulsa **Retirar agente** en el panel y después:

```bash
omarchy plugin remove seamlesscontrol.control
```

En Windows, escoge **Exit SeamlessControl** en la bandeja y borra la carpeta con los dos ejecutables. `%LOCALAPPDATA%\SeamlessControl` se conserva para mantener la identidad y los emparejamientos tras una actualización; bórrala también solo si deseas crear una identidad nueva. Puedes quitar las reglas en Firewall de Windows por sus nombres `SeamlessControl TCP … Private LAN` y `SeamlessControl UDP 5353 Private LAN`.

## Documentación y comunidad

[Guía Omarchy](docs/USER-GUIDE.es.md) · [Guía Windows](docs/WINDOWS.es.md) · [Guía técnica](docs/TECHNICAL.es.md) · [Registro de verificación](docs/TEST-RESULTS.es.md) · [Ayuda](SUPPORT.es.md) · [Contribuir](CONTRIBUTING.es.md) · [Normas de conducta](CODE_OF_CONDUCT.es.md) · [Seguridad](SECURITY.es.md) · [Novedades](CHANGELOG.es.md)

Powered by JhonnySuarez - PuroDelphi. Si SeamlessControl te resulta útil, apoya su desarrollo continuo mediante [GitHub Sponsors](https://github.com/sponsors/PuroDelphi) o [PayPal](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ).

[![QR para donar por PayPal](docs/images/paypal-qr.png)](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ)
