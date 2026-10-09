# SeamlessControl · improvement tracker (alpha)

This document tracks work on the Omarchy ↔ Windows experience. Update the status and evidence as each item is implemented and verified. Keep `main` unchanged until the changes are ready for release.

Status: **planned** → **in progress** → **implemented** → **verified**. An item is complete only after both Omarchy and Windows behavior relevant to it have been checked. Physical two-computer tests are recorded separately from automated tests.

| Priority | Improvement | Status | Completion criteria |
| --- | --- | --- | --- |
| 1 | Connection diagnosis and repair in both interfaces | Implemented | Show agent, receiver, reachable control port, pairing and discovery results with a specific next action; do not claim that an open port proves the peer identity. Physical Omarchy ↔ Windows UI verification remains before marking verified. |
| 2 | Clear and safer updates | Implemented | Show plugin and agent versions in Omarchy, detect mismatch and confirm update completion; offer a Windows package/update flow that replaces the GUI and agent together without losing preferences. Physical UI verification remains before marking verified. |
| 3 | Recover control through lock, sleep and disconnect | Implemented | Bound input-capture portal release on source lock, restore local input and require a fresh edge crossing after recovery. Physical lock, sleep and disconnect tests on both systems remain. |
| 4 | Copy multiple files and folders | Implemented | Offer a group as one approval with item count, total size and safe destination handling; preserve the existing single-file behavior. Physical Omarchy ↔ Windows paste test remains. |
| 5 | Configurable edge crossing | Implemented | Offer an intentional crossing gesture and a full-screen safeguard without weakening Escape or remote return. Physical UI verification remains. |
| 6 | Per-computer permissions and activity | Implemented | Show last connection and allow separate control, text and file permissions per paired identity; revocation must remain immediate. Physical cross-computer checks remain. |

## Verification record

| Date | Item | Evidence | Result |
| --- | --- | --- | --- |
| 2026-10-09 | Baseline | `alpha` starts at `838d164`; current Linux and Windows x64 CI passed for v0.23.3. | Baseline established. |
| 2026-10-09 | Diagnosis | Added a shared, read-only LAN port and local trust check, with an action in each interface. A reachable port is explicitly not treated as proof of identity. Linux: 77 core tests passed; direct CLI probe of the live receiver returned `CHECK … false true pair_first`; Omarchy preview loaded; Windows x64 CI passed at `eda1360`. | Implemented; physical Windows UI check pending. |
| 2026-10-09 | Updates | Embedded manifest version in both agents. Omarchy compares plugin and agent; Windows compares app and agent. The bundled Windows updater is recorded below. | Implemented; physical UI check pending. |
| 2026-10-09 | Omarchy setup result | The setup wrapper now records success, failure or removal. The panel confirms installation only after that result and a matching agent version. Isolated shell integration test covers all three outcomes. | Implemented; live panel check pending. |
| 2026-10-09 | Windows bundled updater | Added Settings button to choose the tagged release ZIP with its adjacent SHA256 asset. A detached updater validates the digest and exact two-file contents, waits for app exit, replaces both executables with rollback, preserves user data and reopens the app with a result message. Windows CI includes success and checksum-rejection scenarios. | Implemented; Windows CI passed; physical UI check pending. |
| 2026-10-09 | Omarchy lock fallback | Bounded portal disable and cleanup to 500 ms when the source locks. If the compositor does not answer, the capture loop exits so reconnect waits for unlock. Direct mode continues to require moving clear of the edge before recapture. | Implemented; Linux and Windows CI passed; physical lock/sleep check pending. |
| 2026-10-09 | Crossing options | Added fluid, deliberate (two crossings within 1.6 seconds), and fullscreen-only deliberate modes to the shared agent, Windows app and Omarchy panel. Escape and receiver return path remain independent. | Implemented; Linux and Windows CI passed; physical crossing check pending. |
| 2026-10-09 | Peer permissions | Saved control/text/files choices by pinned public key on both systems. Both UIs show the choices and last authenticated control connection. New control claims and file offers enforce the relevant permission; direct and mesh text forwarding enforce the text choice. Revocation keeps its separate immediate stop. | Implemented; Linux and Windows CI passed; physical UI checks pending. |
| 2026-10-09 | Copied-file groups | Added one bounded SCB1 package for a multi-file selection or folder, with 256-entry and configured-size limits, portable path checks, individual hashes and one existing encrypted offer/approval. Receiver publishes one verified folder to the local file clipboard; a single file still uses the original protocol. Source packages now count toward the staging quota and are pruned after seven days. Four bundle tests, the quota test and an encrypted loopback test with exactly one approval and rejection passed on Linux. The same portable-core test and complete x64 packaging passed in [Windows CI](https://github.com/PuroDelphi/seamlesscontrol/actions/runs/37938380915); [Linux validation](https://github.com/PuroDelphi/seamlesscontrol/actions/runs/37938380876) and local Clippy also passed. | Implemented; physical paste in both directions pending. |
| 2026-10-09 | Physical test availability | The other LAN test computer did not respond during the verification check. No physical cross-computer result is inferred from the automated tests. | Physical checks remain pending. |

## Implementation notes

- Keep the network protocol and authenticated pairing shared by both platforms.
- Use the existing agent and UI backends; do not replace working input capture or file transfer with a second implementation.
- Changes involving lock and multi-file transfers need physical Omarchy ↔ Windows verification before release.

## Remaining verification

1. Install the latest `alpha` agent/app on a physical Omarchy and Windows pair. Check diagnosis, Windows updater and per-computer permissions in both interfaces.
2. Lock, unlock, suspend and disconnect each physical source while controlling the other computer; verify local input returns and a fresh edge crossing is required.
3. Copy several files and a folder in both directions, approve once, paste the received folder, and verify names and contents. Repeat with rejection, configured size limit and a symbolic link.
4. Exercise all three crossing modes, including a full-screen app, on the physical pair.
