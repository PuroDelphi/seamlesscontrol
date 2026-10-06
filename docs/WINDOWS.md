# SeamlessControl on Windows x64

[Español](WINDOWS.es.md) · [Home](../README.md) · [Technical guide](TECHNICAL.md)

The Windows app uses the same trusted identities and control protocol as the Omarchy plugin. It can receive control from Omarchy, control Omarchy from the physical Windows mouse and keyboard, synchronize text clipboard content and send or receive approved files. The window can be hidden in the system tray while the agent stays active.

![Windows overview with the last computer address and screen edge restored](images/windows-home-en.png)

![Windows screen layout with an example paired computer on the right](images/windows-layout.png)

The screenshots render the app's current interface with fictional names, files and addresses.

## Install

1. Open the [latest release](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), read its installation notes, and under **Assets** download **`seamlesscontrol-windows-x64.zip`**. Its adjacent `.sha256` file lets you verify this one download.
2. Extract the ZIP into a folder you own, such as Downloads or Documents. Both executables are already inside it together. To verify the download in PowerShell, run `Get-FileHash .\seamlesscontrol-windows-x64.zip -Algorithm SHA256` and compare its hash with `seamlesscontrol-windows-x64.zip.sha256` from the same release.
3. Double click `seamlesscontrol.exe`. On its first launch, **Receive control** starts on TCP `47832` and advertises this Windows computer to nearby SeamlessControl panels. Later launches restore the control mode you last chose. If Windows asks about network access, choose **Private networks**.
4. Close the window to leave it running in the tray. Click the tray icon to open it again. Choose **Exit SeamlessControl** from the tray menu to end sessions and exit.

For an `alpha` test build from GitHub Actions, download the single **`seamlesscontrol-windows-x64`** artifact. GitHub gives you one ZIP containing `seamlesscontrol.exe`, `seamlesscontrold.exe` and `SHA256SUMS.txt`; the last file lists the checksums of the two executables. Tagged releases instead provide a checksum for the complete ZIP as an adjacent asset.

To open it automatically at your next Windows sign-in, go to **Settings → Start with Windows** and select **Turn on**. The app starts in the tray and restores your last chosen control mode. This setting belongs only to your Windows account and needs no administrator approval. Select **Turn off** in the same card to remove it. If you move the folder containing the two executables, reopen the app from its new location and select **Turn on** again to update the saved path.

Windows 11 typically includes WebView2; if the app says it is missing, install [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). The app and agent are currently distributed as unsigned executables.

## Pair once

1. Make sure both computers are on the same private LAN. On the **receiver**, leave **Receive control** active. Windows starts it on first launch and restores it on later launches only if it was left active; on Omarchy, select **Receive control** in the panel.
2. On the computer with your physical mouse, open **Computers**. Select the nearby receiver and choose **Pair**. If it does not appear, enter its private LAN `IP:47832` under **Manual address**.
3. Compare the **six digit pairing code** shown on both computers. Approve it on both. In Windows, enter the six digits shown in the app and choose **Codes match · approve**. The longer identity fingerprint is different; it is not a code you choose or edit.
4. The computer now appears as paired. Discovery only supplies an address; the identity check and code still authorize trust.

If an IP was reused by a different computer and the app reports **PeerKey changed**, check which machine owns that address. The old identity is deliberately protected. [Technical details](TECHNICAL.md) explain how peer keys are stored.

To remove a pairing on Windows, open **Computers → Paired computers**, select **Revoke** beside that computer, read the warning, and confirm **Yes, revoke**. This blocks its saved identity, removes its map position and closes an active control session. Do this only when you no longer trust that identity: pairing the same computer again requires changing its identity first. The equivalent manual command in PowerShell, from the app folder, is `.\seamlesscontrold.exe revoke 192.168.1.25`.

## Omarchy controls Windows

On Omarchy, put Windows on the correct side of **Computer layout**. Select **Connect** for the paired Windows computer and wait for **Ready**. Cross the **outer edge** of your Omarchy display in that direction. The Windows mouse and keyboard now respond to Omarchy input. To return, cross the entry edge in Windows or press **Escape** on the physical Omarchy keyboard. Stop the session from the Omarchy panel when finished.

## Windows controls Omarchy

On Omarchy, select **Receive control** and wait for **Available**. In the Windows app, open **Computers**. Paired computers appear first, directly above **Screen layout**; the arrow points from that list to the map. Drag the Omarchy row to its place around **This Windows**, click its position after selecting the row, or focus the row and press an arrow key. Select **Connect** on its row: **Overview** opens with its address and saved edge. Select **Connect** there. When the activity log says **Ready to control**, cross that outer edge. Return by crossing the entry edge on Omarchy or pressing **Escape** on the physical Windows keyboard. Select **Stop** to end the session. The map remains available while this Windows computer is receiving control.

A Windows app can receive or initiate control. Starting **Connect** pauses this app's receiver so the physical Windows keyboard has one clear owner. The app remembers the last `IP:port` and screen edge; they appear again in **Overview → Control another computer** when it reopens. If **Connect** was left active, the app reconnects to that paired computer on launch and keeps retrying if it is unavailable. **Stop** ends that outgoing session and switches to **Receive control**. Stop **Receive control** too if you want the app to reopen without a control session. Returning with Escape or across the screen edge keeps the connection ready for another crossing.

## Clipboard and files

While a control session is active, copy text on one computer and paste it on the other. Text can synchronize in either direction; images and rich clipboard formats are outside this feature.

To receive a file on Windows, open **Files**, choose the destination folder, and select **Wait for a file**. The sender must be paired and must send to the Windows `IP:47833` using its **Files** screen. Review the offer in Windows and choose **Accept file**. Reception waits for one offer; select **Wait for a file** again for the next one. To send from Windows, start **Wait for a file** on Omarchy, enter its `IP:47833` in the Windows **Files** screen, choose a file and select **Send file**. The recipient approves it.

### Copy in one file manager, paste in the other

In **Files**, use the top **Copy here, paste there** card. The **Wait for a file** button below belongs to manual sending on port `47833` and is not needed for a copied file.

![Windows Files screen with a copied file ready to offer](images/windows-files-en.png)

With both apps running and computers paired, copy **one local file** in Explorer or the Omarchy file manager. If there is one paired peer, the app offers it automatically; otherwise choose a destination under **Files → Copy here, paste there**. The receiver must allow TCP `47834` on its private LAN. A native Windows approval dialog appears even with the app hidden in the tray; the Omarchy receiver shows an actionable desktop notification across workspaces. Accept or decline. After the verified transfer, open the destination folder and use **Paste**. No control session or manual **Wait for a file** is needed for this flow. The per-computer file size limit applies on both ends. An unanswered offer expires after two minutes. Transfer progress appears in the app. One file at a time is supported.

Under **Settings → Incoming file approval**, choose **Ask every time** (default), **Accept automatically**, or **Ask, then accept for a while** (1–1,440 minutes). Timed mode asks for the first file from each paired computer; accepting it opens that computer's temporary window. When the time ends or the app restarts, the setting visibly returns to **Ask every time**. A saved confirmation appears after you select **Save approval mode**. The same preference applies to manual file receiving. Windows now shows sender, size and paste instructions in one approval dialog; completion appears in **Activity** without a second dialog. See the [full approval guide](USER-GUIDE.md#choose-how-incoming-files-are-approved).

## Firewall and discovery

Use **Settings → Windows firewall** when another computer cannot reach this Windows receiver. **Allow control port** requests Windows administrator approval for inbound TCP on the currently displayed control port. **Allow file port** does the same for the manual transfer port. **Allow pasted-file port 47834** enables copied-file offers. If Windows does not appear automatically in Omarchy, **Allow discovery · UDP 5353** permits local mDNS queries. The generated rules are limited to the **Private** network profile and **LocalSubnet** addresses. If you change a TCP port, change it in the relevant field before pressing its firewall button. The app reports that approval was *requested*; the elevated Windows console reports the result. You can remove rules named `SeamlessControl TCP … Private LAN` and `SeamlessControl UDP 5353 Private LAN` in Windows Firewall.

![Windows Settings screen with file approval, Start with Windows and private LAN firewall actions](images/windows-settings-en.png)

Discovery uses mDNS on the private LAN. It does not replace the pairing code. If routers, Wi-Fi isolation or multicast filtering hide a computer, enter its private `IP:port` manually. If pairing times out, verify that the receiver is active and its control port is permitted. File transfers need the separate file port permitted on the file **receiver**.

## Update or remove

To update, choose **Exit SeamlessControl** from the tray, download `seamlesscontrol-windows-x64.zip` from the [latest release](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), extract it and replace both `.exe` files in the same folder and open the app again. Existing pairing data under `%LOCALAPPDATA%\SeamlessControl` stays intact.

To remove it, first select **Settings → Start with Windows → Turn off**, then exit from the tray and delete the folder containing the executables. `%LOCALAPPDATA%\SeamlessControl` keeps your local identity and pairings for a later reinstall. Delete that data folder too only if you want a new identity; other computers will then need to pair again. Remove any firewall rules you authorized in Windows Firewall.

For manual commands, security design and physical verification, see the [technical guide](TECHNICAL.md) and [test record](TEST-RESULTS.md).
