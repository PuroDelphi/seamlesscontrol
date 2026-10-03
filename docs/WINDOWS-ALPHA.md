# Windows x64 agent · alpha

[Español](WINDOWS-ALPHA.es.md) · [Omarchy guide](../README.md) · [Test results](TEST-RESULTS.md)

The Windows console agent uses the same pinned Noise identity, pairing code, protocol version and file-transfer format as Omarchy. This alpha targets **64-bit Windows 10/11 on x86-64**. It currently lets an Omarchy computer control the Windows desktop and return by moving back across the Windows entry edge. Windows can also pair and send or receive an approved file. Windows as the **source** of mouse and keyboard input, automatic discovery, clipboard synchronization and a graphical Windows panel are next steps.

## Get the x64 build

Open the [verified Windows x64 build](https://github.com/PuroDelphi/seamlesscontrol/actions/runs/37072862471) from `alpha`. At the bottom of the page, under **Artifacts**, download `seamlesscontrol-windows-x64-alpha`. Extract `seamlesscontrold.exe` and its adjacent `.sha256` file to a folder owned by your Windows account. There is no installer: run the executable from that folder. The workflow builds it natively for `x86_64-pc-windows-msvc` with a static C runtime; it is an unsigned test artifact. In PowerShell, `(Get-FileHash -Algorithm SHA256 .\seamlesscontrold.exe).Hash.ToLowerInvariant()` should match the first field of the `.sha256` file.

You can also build from the `alpha` source on a Windows x64 computer with Rust installed:

```powershell
cargo build --locked --manifest-path agent/Cargo.toml --target x86_64-pc-windows-msvc --release --bin seamlesscontrold
```

## Connect Omarchy to Windows

1. Put both computers on the same private LAN. In Windows, open PowerShell in the folder where you extracted the `.exe` and start the receiver: `.\seamlesscontrold.exe serve 0.0.0.0:47832`. Keep the terminal open. Windows may ask to allow network access; choose **Private networks** only. If a private firewall rule is needed, use the rule below in an **Administrator PowerShell** window after reviewing its scope.
2. On Windows, run `ipconfig` in a second terminal and note the LAN **IPv4 address**. On Omarchy, open SeamlessControl. In **Computers on the network**, enter that address with port `47832` in the manual `IP:port` field, then choose **Pair**. Windows does not announce itself through mDNS yet.
3. Both computers display a six digit code. Compare them. Enter that exact code in the Windows terminal and approve it in the Omarchy panel. The long identity fingerprint is a different value and cannot replace the pairing code.
4. Once paired, under **Computer layout** on Omarchy, place Windows next to **This computer** on the side you will cross. Open **More**, enter the paired Windows `IP:47832` under **Connect by IP**, and select that button. Wait for **Ready**, then cross the chosen outside edge with the physical mouse. To return, move the cursor at least 17 pixels into Windows and then cross back over the edge where it entered; **Escape** on the Omarchy keyboard is another return path.

The Windows receiver infers its return edge from the authenticated `BEGIN` message. Its monitor geometry currently uses the virtual desktop bounds; test one ordinary monitor layout first. Keep an Omarchy terminal accessible while testing so you can stop its connection if needed.

### Optional, scoped Windows firewall rules

Run only if the Windows firewall blocked the receiver and you approve these Private-LAN rules. They allow inbound TCP from the local subnet on the two distinct ports. Port `47833` is needed only when Windows receives a file.

```powershell
New-NetFirewallRule -DisplayName 'SeamlessControl control (Private LAN)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 47832 -Profile Private -RemoteAddress LocalSubnet
New-NetFirewallRule -DisplayName 'SeamlessControl files (Private LAN)' -Direction Inbound -Action Allow -Protocol TCP -LocalPort 47833 -Profile Private -RemoteAddress LocalSubnet
```

Remove them later with `Remove-NetFirewallRule -DisplayName 'SeamlessControl control (Private LAN)'` and the equivalent command for `SeamlessControl files (Private LAN)`.

## File transfer from either side

Pair the two computers first. To receive **one** file on Windows, create a destination folder and run `./seamlesscontrold.exe receive-file 0.0.0.0:47833 C:\Users\YOU\Downloads`. On Omarchy, choose that Windows address in the file sender and use port `47833`. Windows prints the offer and asks for **SI** before writing. Repeat `receive-file` for another file.

To send from Windows to an Omarchy receiver already waiting on its file port, run `./seamlesscontrold.exe send-file OMARCHY_IP:47833 C:\path\to\file.txt`. Approve the offer on Omarchy. The destination checks size and SHA-256 before publishing the file.

## Test scope and safety

- This is a console build intended for a signed-in, unlocked Windows desktop. Before granting control and while applying new input, it checks that the active input desktop is the normal interactive desktop; an unknown or locked state stops injection. Windows can reject `SendInput` into applications running at a higher integrity level; the agent reports the failure and releases its tracked input on a normal disconnect.
- Common physical keys, modifiers, pointer buttons and relative motion are mapped to Windows input. Unmapped keys are logged and ignored. Scroll conversion is approximate and needs a physical test.
- No Windows↔Omarchy physical result is claimed yet. The `alpha` workflow checks the Windows x64 build and portable core; a real Windows machine is needed to verify pairing, crossing, return, shortcuts, files, firewall and lock behavior.
- Peer identities and keys are stored under `%LOCALAPPDATA%\SeamlessControl` for the Windows user. `seamlesscontrold.exe peers` lists trusted peers; `seamlesscontrold.exe revoke PEER_IP` revokes one.

Windows API choices: [SendInput and UIPI](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput), [scan-code keyboard input](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-keybdinput), [input desktop detection](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openinputdesktop), [virtual screen coordinates](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics), and [per-monitor DPI awareness](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocessdpiawarenesscontext).
