# SeamlessControl · improvement tracker (alpha)

This document tracks work on the Omarchy ↔ Windows experience. Update the status and evidence as each item is implemented and verified. Keep `main` unchanged until the changes are ready for release.

Status: **planned** → **in progress** → **implemented** → **verified**. An item is complete only after both Omarchy and Windows behavior relevant to it have been checked. Physical two-computer tests are recorded separately from automated tests.

| Priority | Improvement | Status | Completion criteria |
| --- | --- | --- | --- |
| 1 | Connection diagnosis and repair in both interfaces | In progress | Show agent, receiver, reachable control port, pairing and discovery results with a specific next action; do not claim that an open port proves the peer identity. |
| 2 | Clear and safer updates | In progress | Show plugin and agent versions in Omarchy, detect mismatch and confirm update completion; offer a Windows package/update flow that replaces the GUI and agent together without losing preferences. |
| 3 | Recover control through lock, sleep and disconnect | Planned | Release physical input before a source locks or sleeps; restore local control and require a fresh edge crossing after recovery; test on both physical systems. |
| 4 | Copy multiple files and folders | Planned | Offer a group as one approval with item count, total size and safe destination handling; preserve the existing single-file behavior. |
| 5 | Configurable edge crossing | Planned | Offer an intentional crossing gesture and a full-screen safeguard without weakening Escape or remote return. |
| 6 | Per-computer permissions and activity | Planned | Show last connection and allow separate control, text and file permissions per paired identity; revocation must remain immediate. |

## Verification record

| Date | Item | Evidence | Result |
| --- | --- | --- | --- |
| 2026-10-09 | Baseline | `alpha` starts at `838d164`; current Linux and Windows x64 CI passed for v0.23.3. | Baseline established. |
| 2026-10-09 | Diagnosis | Added a shared, read-only LAN port and local trust check, with an action in each interface. A reachable port is explicitly not treated as proof of identity. | Implemented; Windows CI and physical UI checks pending. |
| 2026-10-09 | Updates | Embedded manifest version in both agents. Omarchy compares plugin and agent; Windows compares app and agent. | Version check implemented; bundled updater pending. |

## Implementation notes

- Keep the network protocol and authenticated pairing shared by both platforms.
- Use the existing agent and UI backends; do not replace working input capture or file transfer with a second implementation.
- Changes involving lock and multi-file transfers need physical Omarchy ↔ Windows verification before release.
