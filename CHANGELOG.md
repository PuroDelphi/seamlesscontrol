# Changelog

[Español](CHANGELOG.es.md) · [All GitHub releases](https://github.com/PuroDelphi/seamlesscontrol/releases)

This file summarizes published changes. Consult the [test results](docs/TEST-RESULTS.md) for verified behavior and remaining test scenarios.

## [0.23.2](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.2) — 2026-10-07

- Bound the Windows control receiver to eight concurrent connections. Excess TCP connections close before creating a worker thread, preventing unauthenticated peers from growing thread and socket use without limit.
- Limit the initial Windows Noise handshake to a ten-second I/O timeout. Restore the longer pairing timeout only after the cryptographic handshake, so a person still has time to compare and approve the six-digit code.
- Add a Windows test for the connection cap and automatic slot release. This corrects the marketplace review finding for 0.23.1; the Omarchy panel and user workflows are unchanged.

## [0.23.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.1) — 2026-10-07

- Dim unavailable Omarchy controls so their disabled state is visible without changing the theme. Explain why **Update agent** is unavailable during an active session.
- Run the Omarchy agent installer from the plugin checkout so Cargo and rustc select the same Rust toolchain when launched from the panel.
- Keep the source Omarchy awake while it controls another computer. Release input capture on a source lock and defer capture barriers while monitor geometry is unavailable after display sleep.
- Add opt-in input tracing to diagnose intermittent remote keyboard delivery. In a physical two-Omarchy session, remote keys worked during the observed period; the intermittent failure was not reproduced.
- In the same physical session, the source remained unlocked after more than five minutes without physical input while control stayed active. After the diagnostic session ended, the user confirmed that its physical mouse and keyboard worked. Manual locking during remote control has not been verified with this revision.

## [0.23.0](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.23.0) — 2026-10-06

- Add pairing revocation to the Windows Computers screen with a confirmation and active-session cleanup.
- Make an explicit Pair request display a fresh six-digit code on both computers, including after revocation. Regular control connections remain blocked until both users approve the new code. Show receiver and firewall guidance beside the Pair actions in both interfaces.
- Return from Omarchy to Windows by crossing the entry edge even when the Omarchy receiver has no saved position for Windows. Verified with a physical Windows-to-Omarchy round trip.
- Arrange pairing and placement as four numbered steps in both apps: pair by address, nearby computers, paired computers, and screen layout. Keep paired computers immediately above the layout and explain how to place them.
- Let an Omarchy receiver stop normally from Home or directly beside the Pair step, even after a shell restart. The emergency input cut remains a pause, and the panel now explains the difference.
- On Windows, outgoing Pair temporarily ends this app's current control session and restores the previous mode when pairing finishes. The receiver on the other computer must still be available. This prevents an already running local agent from blocking pairing.

## [0.22.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.22.1) — 2026-10-05

- Reject drive-relative Windows file names such as `C:payload.txt`, alternate-stream names containing `:`, and any name that is not a single path component before forming a receive destination. An authenticated file offer can no longer redirect publication outside the folder chosen by the receiver.
- Add a Windows x64 regression test for the reported path case. Normal Unicode file names and the existing verified transfer flow remain supported.

## [0.22.0](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.22.0) — 2026-10-05

- Organize the Omarchy panel and Windows tray app around the everyday tasks: receive or control, find and place computers, copy files, and adjust permissions. Keep optional explanations collapsed and put paired Windows computers next to their layout map.
- Copy a file between Omarchy and Windows and paste it on the other computer. Incoming offers remain visible across Omarchy workspaces and Windows virtual desktops. The file is authenticated and verified before Paste.
- Add three receiver approval modes for copied and manually sent files: **Ask every time**, **Accept automatically**, and **Ask, then accept for a while**. Show a saved confirmation and countdown; after the window expires, both interfaces return visibly to **Ask every time**.
- Restore the Windows app's last chosen control mode, address and edge on launch. Start with Windows is optional. Package test builds as one downloadable artifact with both executables and their checksums; the tagged release provides one Windows ZIP and its adjacent SHA256.
- Physical Omarchy/Windows tests passed in both directions for copied files, including Unicode names, duplicate copies, approval, rejection and Paste. The [test record](docs/TEST-RESULTS.md) lists scenarios that still need additional machines or fault injection.

## [0.21.1](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.21.1) — 2026-10-03

- Package the Windows x64 app and agent together in one ZIP with an adjacent SHA-256 file. Installation and update instructions now require a single download.

## [0.21.0](https://github.com/PuroDelphi/seamlesscontrol/releases/tag/v0.21.0) — 2026-10-03

- Add a Windows x64 tray app around the existing console agent, with English and Spanish screens, pairing, discovery, file actions and a draggable computer layout. Embed the tray emblem in the GUI executable and place paired computers directly above the layout with a drag guide.
- Build and test both Windows x64 executables in the Windows workflow. Physical Windows 11 x64 tests verified pairing, automatic LAN discovery in Omarchy, control in both directions, edge and Escape return, clicks, tested key combinations and an Omarchy-to-Windows file transfer.
- Synchronize text clipboard content in both directions between Omarchy and Windows; the user confirmed both directions on physical computers.
- Refresh the preview and bilingual guides to show mouse, keyboard, clipboard, file transfer and the shared Omarchy/Windows workspace.
- Clarify manual Windows pairing and same-IP identity conflicts in the panel.
- Publish the tagged Windows x64 app and agent beside their SHA-256 checksums in GitHub Releases; the user guides now point to the release assets.

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
