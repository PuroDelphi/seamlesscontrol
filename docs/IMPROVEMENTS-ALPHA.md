# SeamlessControl · improvement tracker (alpha)

This document tracks work on the Omarchy ↔ Windows experience. Update the status and evidence as each item is implemented and verified. Keep `main` unchanged until the changes are ready for release.

Status: **planned** → **in progress** → **implemented** → **verified**. An item is complete only after both Omarchy and Windows behavior relevant to it have been checked. Physical two-computer tests are recorded separately from automated tests.

| Priority | Improvement | Status | Completion criteria |
| --- | --- | --- | --- |
| 1 | Connection diagnosis and repair in both interfaces | Implemented | Show agent, receiver, reachable control port, pairing and discovery results with a specific next action; do not claim that an open port proves the peer identity. Physical Omarchy ↔ Windows UI verification remains before marking verified. |
| 2 | Clear and safer updates | Implemented | Show plugin and agent versions in Omarchy, detect mismatch and confirm update completion; offer a Windows package/update flow that replaces the GUI and agent together without losing preferences. Windows CI and physical UI verification remain before marking verified. |
| 3 | Recover control through lock, sleep and disconnect | In progress | Release physical input before a source locks or sleeps; restore local control and require a fresh edge crossing after recovery; test on both physical systems. |
| 4 | Copy multiple files and folders | Planned | Offer a group as one approval with item count, total size and safe destination handling; preserve the existing single-file behavior. |
| 5 | Configurable edge crossing | Implemented | Offer an intentional crossing gesture and a full-screen safeguard without weakening Escape or remote return. Physical UI verification remains. |
| 6 | Per-computer permissions and activity | Implemented | Show last connection and allow separate control, text and file permissions per paired identity; revocation must remain immediate. Windows CI and physical cross-computer checks remain. |

## Verification record

| Date | Item | Evidence | Result |
| --- | --- | --- | --- |
| 2026-10-09 | Baseline | `alpha` starts at `838d164`; current Linux and Windows x64 CI passed for v0.23.3. | Baseline established. |
| 2026-10-09 | Diagnosis | Added a shared, read-only LAN port and local trust check, with an action in each interface. A reachable port is explicitly not treated as proof of identity. Linux: 77 core tests passed; direct CLI probe of the live receiver returned `CHECK … false true pair_first`; Omarchy preview loaded; Windows x64 CI passed at `eda1360`. | Implemented; physical Windows UI check pending. |
| 2026-10-09 | Updates | Embedded manifest version in both agents. Omarchy compares plugin and agent; Windows compares app and agent. | Version check implemented; bundled updater pending. |
| 2026-10-09 | Omarchy setup result | The setup wrapper now records success, failure or removal. The panel confirms installation only after that result and a matching agent version. Isolated shell integration test covers all three outcomes. | Implemented; live panel check pending. |
| 2026-10-09 | Windows bundled updater | Added Settings button to choose the tagged release ZIP with its adjacent SHA256 asset. A detached updater validates the digest and exact two-file contents, waits for app exit, replaces both executables with rollback, preserves user data and reopens the app with a result message. Windows CI includes success and checksum-rejection scenarios. | Implemented; CI and physical UI check pending. |
| 2026-10-09 | Omarchy lock fallback | Bounded portal disable and cleanup to 500 ms when the source locks. If the compositor does not answer, the capture loop exits so reconnect waits for unlock. Direct mode continues to require moving clear of the edge before recapture. | In progress; CI and physical lock/sleep check pending. |
| 2026-10-09 | Crossing options | Added fluid, deliberate (two crossings within 1.6 seconds), and fullscreen-only deliberate modes to the shared agent, Windows app and Omarchy panel. Escape and receiver return path remain independent. | Implemented; Windows CI and physical crossing check pending. |
| 2026-10-09 | Peer permissions | Saved control/text/files choices by pinned public key on both systems. Both UIs show the choices and last authenticated control connection. New control claims and file offers enforce the relevant permission; direct and mesh text forwarding enforce the text choice. Revocation keeps its separate immediate stop. | Implemented; Windows CI and physical UI checks pending. |

## Implementation notes

- Keep the network protocol and authenticated pairing shared by both platforms.
- Use the existing agent and UI backends; do not replace working input capture or file transfer with a second implementation.
- Changes involving lock and multi-file transfers need physical Omarchy ↔ Windows verification before release.

## Next implementation steps

1. **Lock recovery:** trace the current portal capture release on source lock and sleep, then add a bounded fallback that restores physical input without relying on a live remote connection. Keep the source from immediately recapturing after unlock. Verify lock, unlock, suspend and network loss on physical computers.
2. **File groups:** define a bounded manifest with item count, total size and relative paths. Authenticate that manifest, ask once for the entire group, reject absolute paths, traversal and symlinks, and publish files to the clipboard only after all items verify. Preserve the one-file wire path for older paired agents.
3. **File groups:** extend the authenticated file protocol with a bounded group manifest and one approval; keep old one-file transfers compatible.
