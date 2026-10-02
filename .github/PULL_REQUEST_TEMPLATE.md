## What changed / Qué cambió

Describe the user-visible result and link a related issue if there is one. / Describe el resultado visible y enlaza un issue relacionado, si existe.

## Evidence / Pruebas

- [ ] `omarchy plugin validate .`
- [ ] `cargo fmt --manifest-path agent/Cargo.toml --check`
- [ ] `cargo test --locked --manifest-path agent/Cargo.toml --all-targets`
- [ ] `cargo clippy --locked --manifest-path agent/Cargo.toml --all-targets -- -D warnings`
- [ ] Panel screenshots in English and Spanish if the visual interface changed / Capturas del panel en inglés y español si cambió la interfaz
- [ ] Physical test details or a clear note that they are pending / Detalles de pruebas físicas o indicación clara de que faltan

## Release impact / Impacto en la publicación

List changes to permissions, network ports, dependencies, saved data or installation. State whether both language guides and `Cargo.lock` were updated where needed. / Indica cambios en permisos, puertos, dependencias, datos guardados o instalación; comprueba ambas guías y `Cargo.lock` cuando corresponda.
