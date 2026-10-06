# SeamlessControl

[![Validate](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml/badge.svg)](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml) · [Latest release and Windows x64 downloads](https://github.com/PuroDelphi/seamlesscontrol/releases/latest) · [MIT](LICENSE) · [Español](README.es.md)

**One mouse. One keyboard. Every screen in reach.** Move the pointer across an edge to work on the next computer, then cross back or press **Escape** to return. SeamlessControl brings Omarchy and Windows together on your private network, with the same pairing and control protocol on both systems.

![Omarchy and Windows screens connected by mouse, keyboard, clipboard and files](preview.png)

![Omarchy computers and a Windows computer in one workspace](docs/images/ecosystem.svg)

- **Move naturally:** pointer, clicks, scroll and keyboard travel with you. Windows/Super shortcuts work on the destination.
- **Keep your flow:** text clipboard synchronization, return by screen edge or Escape, and local input recovery if the connection ends.
- **Find computers nearby:** LAN discovery shows available receivers. Manual `IP:port` remains available when discovery is blocked.
- **Pair securely:** compare the six digit code on both computers. Each computer remembers the other's identity; a changed key requires review.
- **Copy and paste files across computers:** copy one file in Explorer or the Omarchy file manager, approve the visible offer on the receiver, then paste into the destination folder.
- **Choose how files are approved:** ask every time, accept automatically from paired computers, or approve once for a chosen number of minutes. The temporary option shows a countdown and returns to asking when it expires.
- **Use the interface you expect:** a focused Omarchy panel and a Windows app that stays in the system tray. Both offer English and Spanish, clear save feedback and visible incoming-file prompts.

Use your Omarchy and Windows computers in one workspace: control crosses in either direction, while trusted pairing, discovery and text clipboard keep the session familiar. SeamlessControl is developed continuously. See the [technical guide](docs/TECHNICAL.md) for architecture, permissions and verification records.

**Verified on real Omarchy and Windows 11 x64 computers:** mouse and keyboard control in both directions, return by edge and Escape, text clipboard in both directions, and approved file copy and Paste in both directions. An incoming file prompt appeared on another Omarchy workspace and another Windows virtual desktop; rejecting an offer transferred no file. The [test record](docs/TEST-RESULTS.md) separates these observations from checks still awaiting more computers.

## Simple screens, clear decisions

**Omarchy** keeps installation and the active session on **Home**. **Computers** guides pairing in four steps: address, nearby receivers, paired computers, then layout. If this computer is already receiving, **Stop receiving to pair** sits beside step 1; the emergency input cut only pauses the receiver. **Files** handles copy and paste or a deliberate send; **Settings** holds approval and firewall controls. Optional help opens when you need it.

![The streamlined Omarchy Home screen](docs/images/omarchy-overview-en.png)

**Windows** uses **Overview, Computers, Files and Settings**. Its Computers page follows the same four steps, with paired computers immediately above the draggable layout. When Windows starts an outgoing pairing, the app briefly stops its local control session and restores your previous mode afterward. It also remembers the last control mode, address and screen edge. Close the window to keep it available in the tray.

![Windows pairing steps and screen layout](docs/images/windows-devices-en.png)

For an incoming file from a **paired** computer, choose the level of interruption that suits you:

| Approval mode | What happens |
|---|---|
| **Ask every time** | Review the sender, file and size in one visible prompt. |
| **Accept automatically** | Receive authenticated files without a prompt. |
| **Ask, then accept for a while** | Approve the first file, then accept that computer's files until the countdown ends. The setting returns to **Ask every time**. |

The received file is verified before it becomes available to **Paste**. The prompt reaches the active Omarchy workspace or Windows virtual desktop even when the main panel is closed. [See the illustrated file guide](docs/USER-GUIDE.md#choose-how-incoming-files-are-approved).

## Get started on Omarchy

On every Omarchy computer, install the plugin with the standard command:

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
```

Open **SeamlessControl** from the Omarchy bar. Under **Set up this computer**, choose **Install agent**; the panel installs the required packages and agent after asking for system authorization when needed. The panel opens in English; choose **Español** at the top if you prefer it.

![Agent setup in the Omarchy panel](docs/images/setup-en.png)

## Get started on Windows x64

Open the [latest release and its installation notes](https://github.com/PuroDelphi/seamlesscontrol/releases/latest). Under **Assets**, download the single **`seamlesscontrol-windows-x64.zip`** and extract it into a folder you own. It already contains both required executables together. Double click **`seamlesscontrol.exe`**. The adjacent `seamlesscontrol-windows-x64.zip.sha256` lets you verify the download. On first launch it starts receiving on port `47832`; later it restores your last control mode, including an outgoing connection. It stays active in the tray when you close the window. Choose **Español** in the top right if desired.

For hands-free startup, turn on **Settings → Start with Windows** in the app. It will open in the tray when you sign in; you can turn it off from the same screen.

![The SeamlessControl Windows app with the last connection restored](docs/images/windows-home-en.png)

![Copy a file in Windows and offer it to a paired computer](docs/images/windows-files-en.png)

Windows 11 normally has the required WebView2 runtime. If the app reports it missing, install the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) and open the app again. Windows may ask about network access: allow **Private networks**. In **Settings → Windows firewall**, the app can request administrator approval for private LAN rules on the control port, the file port and mDNS discovery (`UDP 5353`). The TCP buttons follow the ports shown in the app.

[Windows step by step guide](docs/WINDOWS.md) · [Illustrated Omarchy guide](docs/USER-GUIDE.md)

## Connect your screens

1. On the computer you want to control, select **Receive control** and wait for **Available**. The Windows app starts this on first launch and restores it if you left it active.
2. On the computer with the physical mouse, open **Computers**. Use **1 Pair by address** when you know the receiver's private LAN `IP:port`, or **2 Nearby computers** and select **Pair**. Compare the six digit code on **both** computers and approve on both. The pair then appears under **3 Paired computers**. On Omarchy, if this computer is already receiving and you want to start pairing from here, select **Stop receiving to pair** beside step 1; you can start Receive control again afterward.
3. In **4 Screen layout**, place the paired receiver on the side where its screen sits. Both Omarchy and Windows have a visual layout. In Windows, paired computers appear directly above the map: drag one along the arrow, click a side, or use the arrow keys. Its position fills in the edge when you prepare **Connect**.
4. Select **Connect** on the computer with the physical mouse. When it says **Ready**, cross the chosen **outer edge**. Cross back from the receiver or press **Escape** on the physical keyboard to return.

**Pair** records trust once; **Connect** starts a control session. Moving a tile in the layout sets the crossing direction and does not start the session. For file delivery, open **Files** on the receiver, allow its separate LAN port `47833` if needed, select **Wait for a file**, then choose and send a file from the source. The receiver approves the offer. For everyday copying, allow TCP `47834` on the receiving computer, copy one file in Explorer or the Omarchy file manager, approve the prominent incoming prompt, and paste it into the destination folder. This copied-file flow works without an active control session.

![Pair and Connect in the Omarchy panel](docs/images/connect-context-en.png)

## Update and remove

**Omarchy:** stop active sessions, then update the widget on each Omarchy computer:

```bash
omarchy plugin update seamlesscontrol.control --yes
```

`--yes` applies the update without opening the long change preview or asking for confirmation. Run `omarchy restart shell`, reopen SeamlessControl and choose **Update agent** under **Set up this computer**. Your pairing keys and layout are kept.

**Windows:** exit SeamlessControl from its tray menu, download `seamlesscontrol-windows-x64.zip` from the [latest release](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), extract it and replace both executables in their folder and reopen `seamlesscontrol.exe`. Pairing keys in `%LOCALAPPDATA%\SeamlessControl` are kept.

To remove SeamlessControl from Omarchy, choose **Remove agent** in the panel, then:

```bash
omarchy plugin remove seamlesscontrol.control
```

On Windows, choose **Exit SeamlessControl** from the tray and delete the folder containing the two executables. The user data folder `%LOCALAPPDATA%\SeamlessControl` is intentionally kept so your identity and pairings survive an update; delete it too only if you want a completely fresh identity. Firewall rules can be removed in Windows Firewall by their `SeamlessControl TCP … Private LAN` and `SeamlessControl UDP 5353 Private LAN` names.

## Documentation and community

[Omarchy user guide](docs/USER-GUIDE.md) · [Windows user guide](docs/WINDOWS.md) · [Technical guide](docs/TECHNICAL.md) · [Verification record](docs/TEST-RESULTS.md) · [Support](SUPPORT.md) · [Contributing](CONTRIBUTING.md) · [Code of conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md)

Powered by JhonnySuarez - PuroDelphi. If SeamlessControl is useful to you, support its continued development through [GitHub Sponsors](https://github.com/sponsors/PuroDelphi) or [PayPal](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ).

[![PayPal donation QR](docs/images/paypal-qr.png)](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ)
