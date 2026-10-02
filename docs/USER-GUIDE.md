# SeamlessControl user guide

[Back to README](../README.md) · [Español](USER-GUIDE.es.md)

SeamlessControl lets one Omarchy computer (the **source**, with the physical mouse and keyboard) control another (the **receiver**). Set up the plugin and agent on both computers first. All steps below happen in the SeamlessControl panel. The screenshots show fictional computer names and addresses.

## First connection, one step at a time

1. **Start the receiver.** On the computer you want to control, open SeamlessControl. In **Start a session**, leave the address empty and select **Receive control**. The top of the panel should say **Available**. Leave this session running.

   ![Receiver's Receive control button](images/firewall-en.png)

2. **Find and pair it.** On the source, look under **Computers on the network**. Select **Scan** if it is not listed. Select **Pair** beside the receiver. A six digit code appears on **both** computers. Compare the codes and select **Codes match · approve here** on each. You pair a trusted computer once; later sessions reuse its saved key. The much longer **Local identity** is a fingerprint, not the code to type.

   **Pair** authorizes this receiver once. **Connect** starts a new control session whenever you want to use it.

   ![The panel explains Pair and Connect](images/connect-context-en.png)

3. **Place the computers.** In **Computer layout**, place the receiver in the cell beside **This computer** that matches its real position. Do the reverse on the receiver, placing the source beside **This computer**. Select a computer tile, then select the destination cell, or drag it. With the panel open, press Tab to highlight a cell (more presses cycle through the four cells), Enter to select its tile, the arrow keys to reach the target cell, and Enter again to place it. Escape cancels a selection. This only saves the crossing direction; it does not start control.

4. **Connect from the source.** On the source, select **Connect** beside the discovered receiver. Wait for **Ready** at the top. The panel names the edge to cross. If the source has several monitors, use the **outer edge of the whole source desktop**, not a seam between its monitors. Move the pointer through that edge to enter the receiver.

   ![Connect button beside a paired receiver](images/connect-button-en.png)

   ![The source panel shows Ready after the connection starts](images/session-en.png)

5. **Return.** Move through the receiver's edge toward the source, or press **Escape** on the physical keyboard. If the pointer does not return, select **Return control to source** in the receiver's panel. Select **Stop session started here** on the source when finished.

The [README](../README.md) explains installation, updating and removal. Pairing, layout and starting a session are separate actions; changing the layout alone does not connect the computers.

## If the connection does not progress

| What the panel shows | What to do in the panel |
| --- | --- |
| No receiver in **Computers on the network** | Keep **Receive control** active on the receiver, then select **Scan** on the source. For a new computer, enter its `IP:port` in the manual **Pair** field. For an already paired and placed computer, use **Connect by IP**. |
| Pairing times out | On the receiver, use **Firewall · receiver only**: preview the LAN rule for the same port as **Receive control**, then authorize it. Retry **Pair** on the source and approve the new code on both computers. |
| **Connecting**, without a network connection | Check that **Receive control** is still active on the receiver and that its firewall permits the selected port. Stop the source session and select **Connect** again. |
| **Network connected. Preparing mouse and keyboard capture…** for more than 15 seconds | On the source, select **Restart capture on this computer**. This ends the stuck attempt and restarts Omarchy's input capture service. It can interrupt other screen-sharing apps on that computer. When the panel says capture restarted, select **Connect** again. |
| **Ready**, but the pointer does not cross | Check the receiver's position in **Computer layout** on the source. Cross the indicated **outer** edge of all source monitors. If the panel says **Move pointer away from edge**, move inward first, then cross again. |
| **Reconnecting** | The agent retries on its own. Wait a moment and read **Last attempt** for the cause; do not start a second connection while it is retrying. If the receiver stopped or its address or port changed, start **Receive control** there, then select **Stop session started here** on the source, **Scan**, and **Connect** again. A firewall change is only relevant when the connection times out before authentication. |
| Pointer stays on the receiver | Press **Escape** on the source keyboard or select **Return control to source** on the receiver. **Cut remote input · emergency** on the receiver disconnects and pauses it; select **Resume receiving** before another attempt. |

The capture repair button appears only when a panel-started source session has already reached the receiver but its desktop capture setup has stayed pending for 15 seconds. If **Stop session started here** itself hangs, the panel sends a stronger stop signal after two seconds.

## What the other controls do

| Control | Use |
| --- | --- |
| **Connect by IP** | Starts control when the receiver is already paired and placed but discovery does not list it. Enter its `IP:port`. The nearby **Pair** field is for trusting a new computer, not for starting control. |
| **Text clipboard** | While connected, copied text follows control between paired computers automatically. This does not copy files or images. |
| **Connect several computers** | Experimental mesh mode for **one source plus two or three receivers** in a connected 2 × 2 layout. Pair and place every computer; start **Receive control** on each receiver, using the same port. Then select this button on the source. Text clipboard updates are shared between connected receivers. With only two computers in total, use the regular **Connect** button. Mesh still needs physical testing with more than two computers. |
| **Pause capture / Resume capture** | Temporarily stops or resumes edge capture on the source while keeping its session available. |
| **Return control to source** | Requests a return from the receiver while it is being controlled. |
| **Cut remote input · emergency / Resume receiving** | Disconnects remote input at the receiver and blocks new input until you explicitly resume. Use it if the source cannot recover control. |
| **Revoke** | Removes trust for a paired computer. A future connection needs a new pairing and code comparison. |
| **Wait for a file / Send file** | On the receiver, choose a folder and select **Wait for a file**. Wait until the panel shows **Waiting for a file on** with its address. If this is the first file transfer, select **Preview file LAN rule** there, review the rule for port `47833`, then select **Authorize this rule** and approve the system prompt. On the source, choose the paired receiver and a file, then **Send file**. Approve the offer on the receiver. The destination may choose a different file port; enter that same port in the source's receiver address. The default limit is 100 MiB, and existing files are never overwritten. |
| **Preview LAN rule / Authorize this rule** | On the receiver, review and authorize a firewall rule limited to the detected LAN and chosen TCP port. Remote control and file receiving use separate ports and need separate rules when the firewall blocks them. |
| **Update agent / Remove agent** | Installs the current agent version or removes it and packages installed specifically by SeamlessControl. Stop active sessions first. Pairing keys and layout are kept. |

The [technical guide](TECHNICAL.md) covers the protocol, manual diagnostics and current test limits.
