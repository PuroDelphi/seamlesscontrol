# Changelog

[Español](CHANGELOG.es.md) · [All GitHub releases](https://github.com/PuroDelphi/seamlesscontrol/releases)

This file summarizes published changes. Consult the [test results](docs/TEST-RESULTS.md) for verified behavior and remaining test scenarios.

## Unreleased · alpha

- Add a Windows x64 console receiver using the Omarchy pairing and encrypted input protocol. It can request return across its entry edge and send or receive approved files.
- Build and test the Windows x64 executable in an `alpha` workflow. A physical Windows 11 x64 test verified pairing, crossing from Omarchy, edge return, a click, one key, Super+E, and Escape return.
- Clarify manual Windows pairing and same-IP identity conflicts in the panel.

## 0.20.0 — 2026-10-02

- Bring the `alpha` improvements to the published branch: organize the panel into **Main** and **More**, keep essential setup and firewall actions on **Main**, and collapse optional explanations with keyboard access.
- Show file receiver readiness and provide a dedicated LAN firewall action for the file port. Correct file-session handling after an offer and add loopback coverage for the visible listening state.
- Include the promotional preview and the updated English and Spanish guides. One physical file transfer was verified in one direction; the dedicated file-port control and remaining scenarios need further physical testing.

## 0.19.10 — 2026-10-02

- Describe the verified two-computer mouse, keyboard and return-control tests prominently in both READMEs and the marketplace listing.
- Correct the physical file-transfer record: one direction was verified on `alpha`; the other cases remain open. The agent and widget code are unchanged.

## 0.19.9 — 2026-10-02

- Open the file and folder pickers through the desktop portal in a separate agent process, so opening a picker no longer runs GTK/GVfs inside Quickshell. Canceling the external picker was confirmed on the affected Omarchy without a shell crash.
- Validate local file URIs before placing the chosen path in the panel. The manual path fields remain available if the portal fails.

## 0.19.8 — 2026-10-02

- Tried Qt Quick's own dialogs for file and folder selection. The live panel still crashed when **Choose** was pressed; 0.19.9 moves the picker out of Quickshell.

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
