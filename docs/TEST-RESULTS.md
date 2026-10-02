# Current test results

[User guide](../README.md) · [Technical guide](TECHNICAL.md) · [Detailed test log in Spanish](FEASIBILITY.md) · [Planned test cases in Spanish](TESTING.md)

Two physical Omarchy computers on one LAN paired using matching six digit codes on both machines. The receiver initially timed out because its firewall blocked the chosen TCP port; a rule scoped to its local interface, subnet, address and port allowed pairing. The user later confirmed that **Preview LAN rule** and **Authorize this rule** worked from the receiver panel.

The mouse crossed from the source's right edge to the receiver and returned across the receiver's left edge on two separate physical attempts. The user confirmed local mouse control after each return. A physical Escape press also returned control. These results used the same two computers and one monitor arrangement; the development log records the agent revisions and earlier failures.

The original trapped cursor episode involved a source `connect` process in a suspended `T` state, which could not process Escape, return feedback or termination signals. A separate immediate recapture occurred after Escape; the current agent requires the returned pointer to move 96 pixels inside the source before rearming the capture edge. Hyprland's unlocked `solitaryBlockedBy: null` response is also handled when an active workspace is present.

Local loopback and integrated tests cover pairing, Noise XX, discovery, simulated network address changes, file offer/acceptance, return messages, virtual input and capture. The [technical guide](TECHNICAL.md) lists the commands. The firewall helper has a test for rule scope and confirmation, and the setup scripts have tests that preserve identity and remove only tracked packages.

The following still need physical verification: keyboard, wheel and dragging between the two machines; file transfer; clipboard; multiple receiver mesh; other screen edges and monitor arrangements; locking, sleep and network loss. Windows support is planned for a later phase.
