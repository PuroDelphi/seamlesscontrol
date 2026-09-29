# SeamlessControl

Plugin nuevo para Omarchy que busca compartir un teclado y ratón entre equipos. El primer objetivo es Omarchy ↔ Omarchy; después se ampliará a Windows ↔ Omarchy. [Plan completo](./PLAN.md) · [Plan visual](./index.html).

## Estado actual

Se ha validado en un Omarchy 4.0.4 con Hyprland 0.56.2 que el portal `InputCapture` acepta una barrera y entrega una conexión EIS, y que Hyprland permite crear dispositivos virtuales de teclado y puntero sin privilegios de sistema. El agente experimental ya une la captura, el canal Noise XX y la inyección, pero **el recorrido entre dos equipos todavía no se ha probado**. No hay una versión lista para uso cotidiano.

## Probar el agente experimental entre dos Omarchy

En ambos equipos, compile con `cargo build --release --manifest-path agent/Cargo.toml --bin seamlesscontrold`. En el equipo que recibirá el control, ejecute:

```bash
agent/target/release/seamlesscontrold serve 192.168.1.20:47832
```

Sustituya `192.168.1.20` por la IP LAN de ese equipo. En el equipo que tiene teclado y ratón físicos, ejecute:

```bash
agent/target/release/seamlesscontrold connect 192.168.1.20:47832 right
```

`right` puede cambiarse por `left`, `top` o `bottom` según el borde de salida. La primera conexión muestra un código de seis cifras en ambos terminales. **Compare los códigos y escriba `SI` en ambos sólo si coinciden.** Las claves quedan fijadas por IP en `~/.config/seamlesscontrol/peers/`, con permisos privados. La captura puede solicitar consentimiento del portal. Cruce el borde elegido para enviar teclado y ratón al destino; pulse **Escape** para devolver el control local, y **Ctrl+C** en el terminal de origen para cerrar. La conexión termina si fallan los latidos; el receptor libera teclas y botones que hayan quedado pulsados.

Esta prueba requiere dos sesiones Omarchy reales en la misma LAN. Hoy sólo está disponible una; la lista de pruebas físicas está en [docs/TESTING.md](./docs/TESTING.md). El regreso por el borde del equipo remoto, la reconexión automática, la configuración visual de pares, el portapapeles, los archivos y Windows siguen pendientes.

## Widget de Omarchy

El repositorio incluye un `manifest.json` válido y un widget que muestra el estado del prototipo y abre esta guía. Se puede instalar con `omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol --enable`. **El widget todavía no controla el agente**: el emparejamiento y las pruebas se hacen en los terminales como se indica arriba. Esta limitación queda visible dentro del panel.

## Compilar y probar

Con Rust estable y Cargo instalados:

```bash
cargo test --manifest-path agent/Cargo.toml
cargo clippy --manifest-path agent/Cargo.toml --all-targets -- -D warnings
```

La prueba criptográfica usa sockets en `127.0.0.1`; el entorno de pruebas debe permitirlo. En un Omarchy con sesión gráfica se pueden ejecutar los probes:

```bash
cargo run --manifest-path agent/Cargo.toml --bin virtual-input-probe
cargo run --manifest-path agent/Cargo.toml --bin capture-probe
cargo run --manifest-path agent/Cargo.toml --bin capture-probe -- --listen
```

El primer probe crea y cierra dispositivos virtuales sin enviar eventos. El segundo solicita permiso al portal, instala temporalmente una barrera en el borde derecho del primer monitor y comprueba la conexión EIS. Con `--listen`, espera hasta 20 segundos por una activación, lee un evento durante un máximo de cinco segundos y libera la captura. Sólo imprime si llegó un evento, sin registrar su contenido. Consulte [los resultados locales](./docs/FEASIBILITY.md).

## Licencia

MIT. Consulte [LICENSE](./LICENSE).
