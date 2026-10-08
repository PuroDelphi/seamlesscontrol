# Contributing to SeamlessControl

[Español](CONTRIBUTING.es.md) · [Code of conduct](CODE_OF_CONDUCT.md)

Bug fixes, documentation improvements, translations and focused features are welcome. The [test results](docs/TEST-RESULTS.md) distinguish physical checks from simulated ones. Please describe new paths according to the evidence available for them.

## Before you start

1. Search [existing issues](https://github.com/PuroDelphi/seamlesscontrol/issues) and the [pending verification scenarios](docs/TESTING.md). Open an issue for a large change, a protocol change or a new platform so the intended behavior can be discussed first. Small fixes can go straight to a pull request.
2. Read the [technical guide](docs/TECHNICAL.md) and keep end-user flows in the panel. Installation should continue to use `omarchy plugin add`; extra agent dependencies are managed through the panel.
3. Work in a branch from current `main`. Keep unrelated changes in separate pull requests.

## Local checks

```bash
omarchy plugin validate .
cargo fmt --manifest-path agent/Cargo.toml --check
cargo test --locked --manifest-path agent/Cargo.toml --all-targets
cargo clippy --locked --manifest-path agent/Cargo.toml --all-targets -- -D warnings
bash -n packaging/*.sh scripts/*.sh tests/*.sh
```

Some integration tests require a running Omarchy desktop or a second computer. Say which checks you ran, what environment you used and what remains unverified. For panel changes, include an English and Spanish screenshot when the visual result changes. Update both language guides when user-facing behavior changes. Keep `agent/Cargo.lock` committed for dependency changes.

## Maintain guide images

From the repository root:

```bash
python3 scripts/render-omarchy-doc-screenshots.py
python3 scripts/render-windows-doc-screenshots.py
```

The first requires Quickshell and the installed `/usr/share/omarchy/shell` components; `--shell-source` selects another shell checkout and `--output` a review folder. It renders real QML with a fictional-data backend without running the agent or using desktop configuration. The second requires Chromium and renders the Windows app's HTML on Linux without a Windows agent. Both regenerate English and Spanish images and label their provenance.

Inspect the results in rendered guides: readable text, pairing code, layout and buttons without clipping. These images are not physical control, native-dialog or firewall verification. Remove images with no remaining references when changing the guide flow.

## Pull request

Describe the problem, the visible change, the test evidence and any new permissions, ports, packages or persistence. Avoid committing binaries, local identities, pairing data, SSH material, private IP addresses or real user content. The CI workflow must pass before a release. The maintainer may request changes or defer a feature that expands input or network access without enough evidence.

Security findings should use the [private reporting route](SECURITY.md) instead of a public issue or pull request.
