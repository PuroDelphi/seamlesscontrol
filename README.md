# SeamlessControl

[![Validate](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml/badge.svg)](https://github.com/PuroDelphi/seamlesscontrol/actions/workflows/validate.yml) · [Download Windows x64](https://github.com/PuroDelphi/seamlesscontrol/releases/latest) · [Español](README.es.md) · [MIT](LICENSE)

[See what's new in 0.24.4](docs/RELEASE-0.24.4.md) · [0.24.0 feature release](docs/RELEASE-0.24.0.md)

**One mouse and keyboard for your Omarchy and Windows computers on a private LAN.** Cross the outer desktop edge to control another computer; return across the entry edge or press **Escape on the source's physical keyboard**.

You can also share text during a connection and copy **files and folders** between connected computers. The receiver chooses how to approve files; they become available only after verification.

![SeamlessControl: mouse, keyboard, clipboard and files across Omarchy and Windows](preview.png)

- **Move naturally:** cross the screen edge and return the same way. Choose fluid crossing, a deliberate double crossing, or protection while a window is full-screen.
- **Keep control of trust:** compare a six-digit pairing code on both computers, then set Control, Text and Files permissions for each paired computer.
- **Copy a whole group:** select several files or a folder with nested files, approve one offer on the receiver, and paste the verified group there.
- **Share only while connected:** a saved pairing never starts file sharing by itself. Ending the control session ends new file offers and transfers.
- **Recover smoothly:** local input returns after a lock, sleep or stopped receiver; a fresh edge crossing resumes remote control.

## Choose where to start

| I want to… | Where to look |
|---|---|
| Install the plugin and connect from Omarchy | [Illustrated Omarchy guide](docs/USER-GUIDE.md) |
| Install the app, receive or control from Windows | [Illustrated Windows guide](docs/WINDOWS.md) |
| Copy and paste files or send to a chosen folder | [Omarchy guide](docs/USER-GUIDE.md) / [Windows guide](docs/WINDOWS.md), Files section |
| Resolve a connection problem | Troubleshooting in each guide; [get support](SUPPORT.md) |
| See the latest fix and physical verification | [0.24.4 release notes](docs/RELEASE-0.24.4.md) / [test record](docs/TEST-RESULTS.md) |
| Use commands or understand permissions and limits | [Technical guide](docs/TECHNICAL.md) |

The **source** is the computer with the physical mouse and keyboard you will use. The **receiver** is the computer you want to control. Roles can change for another session; reversing them does not require pairing again.

## 1. Install on both computers

### Omarchy

```bash
omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable
```

Open **SeamlessControl** from the bar and select **Home → Set up this computer → Install agent**. A terminal opens to install dependencies and build the agent; authorize installation when the system asks. Wait for it to finish before connecting. The initial language is English; select **Español** at the top if preferred.

![Omarchy plugin Home: agent setup and receiving control](docs/images/omarchy-overview-en.png)

### Windows x64

1. In the [latest release](https://github.com/PuroDelphi/seamlesscontrol/releases/latest), download **`seamlesscontrol-windows-x64.zip`** under **Assets**.
2. **Extract the ZIP** into a folder you own. Keep `seamlesscontrol.exe` and `seamlesscontrold.exe` together; do not run them inside the ZIP.
3. Open **`seamlesscontrol.exe`**. On first launch it receives control on TCP `47832`. Later it restores the last mode you left active, including an outgoing connection.
4. If Windows asks for network access, allow **Private networks** only. If WebView2 is missing, install the [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

Closing the window **does not** exit the app: it stays in the tray. To exit, choose **Exit SeamlessControl** in its tray menu. Automatic startup is optional under **Settings → Start with Windows**.

The [Windows guide](docs/WINDOWS.md) covers SHA-256 verification, firewall setup and installation from a tagged release. Development builds remain available through Actions.

## 2. Pair, place and connect

1. **On the receiver:** activate **Receive control**. In Omarchy, wait for **Available**; in Windows, check that receiving is active on Overview.
2. **On the source:** open **Computers**. Under **1 · Pair by address**, enter the receiver's private LAN `IP:port`; or use **2 · Nearby computers → Pair**. These are alternatives, not two required steps.
3. **On both computers:** compare the **six-digit** code and approve only if it matches. The long identity fingerprint is not this code. The receiver appears under **3 · Paired computers**.
4. **On the source:** place the receiver in **4 · Screen layout**, next to your computer on the side where its physical screen sits. This saves the crossing edge: **it does not connect yet**.
5. **Start the session:** in Omarchy, select **Connect** beside the discovered receiver, or use **Settings → Connect by IP** if it is not listed. In Windows, **Connect** in a computer row prepares Overview; select **Connect again on Overview** to start the session.
6. Once ready, cross the **outer edge of the complete desktop**, not the boundary between monitors on the same computer. Return across the receiver's entry edge or with physical **Escape**. Returning keeps the session ready for another crossing; stopping it is a separate action.

![Windows Computers: pairing alternatives, trusted peers and screen placement](docs/images/windows-devices-en.png)

Images show interfaces with fictional documentation data; each platform guide explains how its images were produced.

**If Omarchy is already receiving and you want to start pairing there**, select **Stop receiving to pair** beside step 1. The emergency cut only pauses: it does not release the receiver for initiating pairing. Windows temporarily stops its own session while pairing and restores the previous mode afterward. In both cases, the **other** computer's receiver must stay active.

## 3. Choose how to share

| Feature | What you need | Port on the receiver |
|---|---|---|
| Mouse, keyboard and text clipboard | Pair + Receive control + Connect | TCP `47832` by default |
| Copy in one file manager and paste in the other | Apps open + pairing + active control connection; approve the offer when required and wait for verification before Paste | TCP `47834` |
| Send to a chosen folder | Active control connection + Files → Wait for a file on the receiver; choose and send on the source | TCP `47833` by default |
| Find nearby receivers | Active receiving and discovery allowed on the LAN | UDP `5353` (mDNS) |

**Both file flows require an active control connection.** Pairing saves trust; stopping control ends sharing. **Wait for a file** does not enable copy and paste: it is the manual send flow, one offer at a time. Opening the control port does not open the file ports.

Under **Settings → Incoming file approval**, you can ask for every offer (default), accept automatically from paired computers, or approve for a while. Temporary mode asks for the first file from each computer and returns to **Ask every time** on expiry or restart. The [illustrated guides](docs/USER-GUIDE.md) explain size limits, destination selection and missing offers.

Do not expose these ports to the Internet or set up router port forwarding. Use the interface's firewall actions on the **receiver**, scoped to the private LAN.

## Update or remove

### Omarchy

End sessions on both computers. Update the plugin and reload the shell:

```bash
omarchy plugin update seamlesscontrol.control --yes
omarchy restart shell
```

Open the panel and select **Update agent**. Do this on each participating Omarchy before resuming. Identity, pairings and layout are preserved.

To remove it, use **Settings → Remove agent**, then `omarchy plugin remove seamlesscontrol.control`. Removal preserves pairing data; firewall rules are removed separately as explained in the [technical guide](docs/TECHNICAL.md).

### Windows

To update, download the release ZIP and its adjacent `.sha256` file into the same folder. Choose **Settings → Update from release ZIP…**; the app checks the checksum, replaces **both executables** and reopens. Data in `%LOCALAPPDATA%\SeamlessControl` is preserved.

To uninstall, first disable **Settings → Start with Windows**, exit from the tray and delete the executable folder. Remove any authorized `SeamlessControl TCP … Private LAN` and `SeamlessControl UDP 5353 Private LAN` rules. Delete the data folder **only** if you want a new identity: you will have to pair again.

## Scope and project documentation

The [test record](docs/TEST-RESULTS.md) covers physical Omarchy ↔ Windows 11 x64 control, return, clipboard and grouped files in both directions, plus lock, sleep and receiver interruption. Multi-computer mesh remains experimental; see the test record for the scope of other layouts and network conditions.

The guides above describe current use. The `docs/RELEASE-*.md` notes, changelog and feasibility log preserve **version history**, not current installation instructions.

[Support](SUPPORT.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Changelog](CHANGELOG.md) · [Code of conduct](CODE_OF_CONDUCT.md)

Powered by JhonnySuarez - PuroDelphi. Support the project through [GitHub Sponsors](https://github.com/sponsors/PuroDelphi) or [PayPal](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ).

[![PayPal donation QR](docs/images/paypal-qr.png)](https://www.paypal.com/donate/?hosted_button_id=KBAUBYYDNHQNQ)
