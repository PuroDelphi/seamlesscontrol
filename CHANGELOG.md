# Changelog

[Español](CHANGELOG.es.md) · [All GitHub releases](https://github.com/PuroDelphi/seamlesscontrol/releases)

This file summarizes user-visible published changes. The repository is experimental; consult the [test results](docs/TEST-RESULTS.md) for verified behavior.

## 0.19.8 — 2026-10-02

- Open the file and folder pickers with Qt Quick's own dialogs. This avoids the GTK/GVfs path that aborted Quickshell when **Choose** was pressed on the tested Omarchy.

## 0.19.7 — 2026-10-02

- Close authenticated mesh links when their peer identity is revoked, even when that peer no longer controls input. Discard queued feedback from revoked links and release capture if the revoked peer was active.
- Added a local encrypted loopback regression test. A physical multi-computer mesh retest is still pending.

## 0.19.6 — 2026-10-02

- Publish a tagged source archive and adjacent SHA256 checksum as GitHub Release assets.
- Automate asset generation and verification for future releases. The agent and installation flow are unchanged.

## 0.19.5 — 2026-10-02

- Preserve existing customized SeamlessControl user service files during agent installation and removal.
- List setup dependencies directly in the README for marketplace review.

## 0.19.4 — 2026-10-02

- Fixed intermittent connection failure when the receiver read the Wayland keyboard keymap from a shared file position.
- Improved the agent error if a compositor keymap still cannot be read. No pairing or network protocol changes.

## 0.19.3 — 2026-10-02

- Show the most recent agent retry error in the panel during reconnection.
- Clarify the steps for transient reconnection and document an observed interruption during an active-session plugin reload. No agent or network protocol changes.

## 0.19.2 — 2026-10-02

- Added bilingual contribution, support, security, conduct and release documentation.
- Added issue and pull request templates and dependency update configuration.
- Improved the repository About, topics and private security reporting. No agent protocol or installation behavior changed.

## [0.19.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.19.1) — 2026-10-02

- Added the creator credit to the panel.
- Published a source-only release at an exact, CI-validated commit SHA.
- Enabled GitHub Sponsors and documented the release and marketplace SHA process.

## 0.19.0 — 2026-10-02

- Updated virtual keyboard modifier state so shortcuts such as Super+V reach the receiving Omarchy. Super+V and pointer return were confirmed with two physical computers.
- Added a locked Cargo build and bilingual publishing guidance.

Earlier development history is available in the [commit log](https://github.com/PuroDelphi/seamlesscontrol/commits/main/).
