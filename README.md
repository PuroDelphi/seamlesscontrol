# SeamlessControl

Plugin nuevo para Omarchy que busca compartir un teclado y ratón entre equipos. El primer objetivo es Omarchy ↔ Omarchy; después se ampliará a Windows ↔ Omarchy. [Plan completo](./PLAN.md) · [Plan visual](./index.html).

## Estado actual

Se ha validado en un Omarchy 4.0.4 con Hyprland 0.56.2 que el portal `InputCapture` acepta una barrera y entrega una conexión EIS, y que Hyprland permite crear dispositivos virtuales de teclado y puntero sin privilegios de sistema. El núcleo Rust incluye encuadre de mensajes, topología, estado de teclas/botones, identidad persistente y un canal Noise XX con verificación de código corto. **Todavía no hay un agente o plugin listo para uso entre equipos.**

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
