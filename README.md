# SeamlessControl

[![Validate](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml/badge.svg)](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml) · [Latest release and Windows x64 downloads](https://github.com/PuroDelphi/seamlesscontrol/releases/latest) · [MIT](LICENSE) · [Español](README.es.md)

**One mouse. One keyboard. Every screen in reach.** Move the pointer across an edge to work on the next computer, then cross back or press **Escape** to return. SeamlessControl brings Omarchy and Windows together on your private network, with the same pairing and control protocol on both systems.

![Omarchy and Windows screens connected by mouse, keyboard, clipboard and files](preview.png)

![Omarchy computers and a Windows computer in one workspace](docs/images/ecosystem.svg)

- **Move naturally:** pointer, clicks, scroll and keyboard travel with you. Windows/Super shortcuts work on the destination.
- **Keep your flow:** text clipboard synchronization, return by screen edge or Escape, and local input recovery if the connection ends.
- **Find computers nearby:** LAN discovery shows available receivers. Manual `IP:port` remains available when discovery is blocked.
- **Pair securely:** compare the six digit code on both computers. Each computer remembers the other's identity; a changed key requires review.
- **Send files deliberately:** the receiver approves each offer; the transferred file is checked before it is published. File reception uses its own port.
- **Use the interface you expect:** an Omarchy panel and a Windows app that stays in the system tray. Both offer English and Spanish.

Use your Omarchy and Windows computers in one workspace: control crosses in either direction, while trusted pairing, discovery and text clipboard keep the session familiar. SeamlessControl is developed continuously. See the [technical guide](docs/TECHNICAL.md) for architecture, permissions and verification records.

## Get started on Omarchy

On every Omarchy computer, install the plugin with the standard command:

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
```

Open **SeamlessControl** from the Omarchy bar. Under **Set up this computer**, choose **Install agent**; the panel installs the required packages and agent after asking for system authorization when needed. The panel opens in English; choose **Español** at the top if you prefer it.

![Agent setup in the Omarchy panel](docs/images/setup-en.png)

## Get started on Windows x64

Open the [latest release and its installation notes](https://github.com/PuroDelphi/seamlesscontrol/releases/latest). Under **Assets**, download the single **`seamlesscontrol-windows-x64.zip`** and extract it into a folder you own. It already contains both required executables together. Double click **`seamlesscontrol.exe`**. The adjacent `seamlesscontrol-windows-x64.zip.sha256` lets you verify the download. It starts receiving on port `47832`, advertises this Windows computer on the LAN and stays active in the tray when you close the window. Choose **Español** in the top right if desired.

![The SeamlessControl Windows app](docs/images/windows-panel.png)

Windows 11 normally has the required WebView2 runtime. If the app reports it missing, install the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) and open the app again. Windows may ask about network access: allow **Private networks**. In **Settings → Windows firewall**, the app can request administrator approval for private LAN rules on the control port, the file port and mDNS discovery (`UDP 5353`). The TCP buttons follow the ports shown in the app.

[Windows step by step guide](docs/WINDOWS-ALPHA.md) · [Illustrated Omarchy guide](docs/USER-GUIDE.md)

## Connect your screens

1. On the computer you want to control, select **Receive control** and wait for **Available**. The Windows app starts this automatically.
2. On the computer with the physical mouse, open **Computers**, select the discovered receiver and choose **Pair**. Compare the six digit code on **both** computers and approve on both. If discovery cannot find it, enter its private LAN `IP:port` manually.
3. In **Computers → Screen layout**, place the paired receiver on the side where its screen sits. Both Omarchy and Windows have a visual layout. In Windows, paired computers appear directly above the map: drag one along the arrow, click a side, or use the arrow keys. Its position fills in the edge when you prepare **Connect**.
4. Select **Connect** on the computer with the physical mouse. When it says **Ready**, cross the chosen **outer edge**. Cross back from the receiver or press **Escape** on the physical keyboard to return.

**Pair** records trust once; **Connect** starts a control session. Moving a tile in the layout sets the crossing direction and does not start the session. For file delivery, open **Files** on the receiver, allow its separate LAN port `47833` if needed, select **Wait for a file**, then choose and send a file from the source. The receiver approves the offer.

![Pair and Connect in the Omarchy panel](docs/images/connect-context-en.png)

## Update and remove

**Omarchy:** stop active sessions, then update the widget on each Omarchy computer:

```bash
omarchy plugin update seamlesscontrol.control
```

Run `omarchy restart shell`, reopen SeamlessControl and choose **Update agent** under **Set up this computer**. Your pairing keys and layout are kept.

**Windows:** exit SeamlessControl from its tray menu, download `seamlesscontrol-windows-x64.zip` from the [latest release](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), extract it and replace both executables in their folder and reopen `seamlesscontrol.exe`. Pairing keys in `%LOCALAPPDATA%\SeamlessControl` are kept.

To remove SeamlessControl from Omarchy, choose **Remove agent** in the panel, then:

```bash
omarchy plugin remove seamlesscontrol.control
```

On Windows, choose **Exit SeamlessControl** from the tray and delete the folder containing the two executables. The user data folder `%LOCALAPPDATA%\SeamlessControl` is intentionally kept so your identity and pairings survive an update; delete it too only if you want a completely fresh identity. Firewall rules can be removed in Windows Firewall by their `SeamlessControl TCP … Private LAN` and `SeamlessControl UDP 5353 Private LAN` names.

## Documentation and community

[Omarchy user guide](docs/USER-GUIDE.md) · [Windows user guide](docs/WINDOWS-ALPHA.md) · [Technical guide](docs/TECHNICAL.md) · [Verification record](docs/TEST-RESULTS.md) · [Support](SUPPORT.md) · [Contributing](CONTRIBUTING.md) · [Code of conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md)

Powered by JhonnySuarez - PuroDelphi. If SeamlessControl is useful to you, support its continued development through [GitHub Sponsors](https://github.com/sponsors/PuroDelphi) or [PayPal](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ).

[![PayPal donation QR](docs/images/paypal-qr.png)](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ)
