# SeamlessControl on Windows x64

[Español](WINDOWS-ALPHA.es.md) · [Home](../README.md) · [Technical guide](TECHNICAL.md)

The Windows app uses the same trusted identities and control protocol as the Omarchy plugin. It can receive control from Omarchy, control Omarchy from the physical Windows mouse and keyboard, synchronize text clipboard content and send or receive approved files. The window can be hidden in the system tray while the agent stays active.

![Windows overview with receiver and connection controls](images/windows-panel.png)

## Install

1. From [Windows x64 builds](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/windows-alpha.yml?query=branch%3Aalpha), open the most recent **successful** run and download the `seamlesscontrol-windows-x64-alpha` artifact.
2. Extract `seamlesscontrol.exe` and `seamlesscontrold.exe` into the **same folder**. The two adjacent `.sha256` files let you compare their hashes if you wish. Keep the folder somewhere you own, such as Downloads or Documents.
3. Double click `seamlesscontrol.exe`. **Receive control** starts automatically on TCP `47832`. The app advertises this Windows computer to nearby SeamlessControl panels. If Windows asks about network access, choose **Private networks**.
4. Close the window to leave it running in the tray. Click the tray icon to open it again. Choose **Exit SeamlessControl** from the tray menu to end sessions and exit.

Windows 11 typically includes WebView2; if the app says it is missing, install [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). The app and agent are currently distributed as unsigned executables.

## Pair once

1. Make sure both computers are on the same private LAN. On the **receiver**, leave **Receive control** active. Windows starts it when the app opens; on Omarchy, select **Receive control** in the panel.
2. On the computer with your physical mouse, open **Computers**. Select the nearby receiver and choose **Pair**. If it does not appear, enter its private LAN `IP:47832` under **Manual address**.
3. Compare the **six digit pairing code** shown on both computers. Approve it on both. In Windows, enter the six digits shown in the app and choose **Codes match · approve**. The longer identity fingerprint is different; it is not a code you choose or edit.
4. The computer now appears as paired. Discovery only supplies an address; the identity check and code still authorize trust.

If an IP was reused by a different computer and the app reports **PeerKey changed**, check which machine owns that address. The old identity is deliberately protected. [Technical details](TECHNICAL.md) explain how peer keys are stored.

## Omarchy controls Windows

On Omarchy, put Windows on the correct side of **Computer layout**. Select **Connect** for the paired Windows computer and wait for **Ready**. Cross the **outer edge** of your Omarchy display in that direction. The Windows mouse and keyboard now respond to Omarchy input. To return, cross the entry edge in Windows or press **Escape** on the physical Omarchy keyboard. Stop the session from the Omarchy panel when finished.

## Windows controls Omarchy

On Omarchy, select **Receive control** and wait for **Available**. In the Windows app's **Overview**, enter the paired Omarchy `IP:47832` under **Control another computer**. Choose the **Windows screen edge** that faces Omarchy and select **Connect**. When the activity log says **Ready to control**, cross that outer edge. Return by crossing the entry edge on Omarchy or pressing **Escape** on the physical Windows keyboard. Select **Stop** to end the session.

A Windows app can receive or initiate control. Starting **Connect** stops this app's receiver so the physical Windows keyboard has one clear owner. Stopping Connect leaves the receiver stopped until you choose **Start receiving** again or exit and relaunch the app.

## Clipboard and files

While a control session is active, copy text on one computer and paste it on the other. Text can synchronize in either direction; images and rich clipboard formats are outside this feature.

To receive a file on Windows, open **Files**, choose the destination folder, and select **Wait for a file**. The sender must be paired and must send to the Windows `IP:47833` using its **Files** screen. Review the offer in Windows and choose **Accept file**. Reception waits for one offer; select **Wait for a file** again for the next one. To send from Windows, start **Wait for a file** on Omarchy, enter its `IP:47833` in the Windows **Files** screen, choose a file and select **Send file**. The recipient approves it.

## Firewall and discovery

Use **Settings → Windows firewall** when another computer cannot reach this Windows receiver. **Allow control port** requests Windows administrator approval for inbound TCP on the currently displayed control port. **Allow file port** does the same for the separate file port. The generated rule is limited to the **Private** network profile and **LocalSubnet** addresses. If you change either port, change it in the relevant field before pressing its firewall button. The app reports that approval was *requested*; the elevated Windows console reports the result. You can remove rules named `SeamlessControl TCP … Private LAN` in Windows Firewall.

Discovery uses mDNS on the private LAN. It does not replace the pairing code. If routers, Wi-Fi isolation or multicast filtering hide a computer, enter its private `IP:port` manually. If pairing times out, verify that the receiver is active and its control port is permitted. File transfers need the separate file port permitted on the file **receiver**.

## Update or remove

To update, choose **Exit SeamlessControl** from the tray, download the newest successful Windows x64 artifact, replace **both** `.exe` files in the same folder and open the app again. Existing pairing data under `%LOCALAPPDATA%\SeamlessControl` stays intact.

To remove it, exit from the tray and delete the folder containing the executables. `%LOCALAPPDATA%\SeamlessControl` keeps your local identity and pairings for a later reinstall. Delete that data folder too only if you want a new identity; other computers will then need to pair again. Remove any firewall rules you authorized in Windows Firewall.

For manual commands, security design and physical verification, see the [technical guide](TECHNICAL.md) and [test record](TEST-RESULTS.md).
