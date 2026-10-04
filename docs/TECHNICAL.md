# SeamlessControl technical guide

[User guide](../README.md) · [Español](TECHNICAL.es.md) · [Physical test results](TEST-RESULTS.md) · [Detailed log (Spanish)](FEASIBILITY.md) · [Test plan (Spanish)](TESTING.md) · [Local control protocol (Spanish)](IPC.md)

The [illustrated panel guide](USER-GUIDE.md) covers the ordinary UI connection and recovery flow.

The [Windows user guide](WINDOWS.md) covers the tray app. On Windows x64, `seamlesscontrol.exe` is a Wry/WebView2 and Tao shell around the existing `seamlesscontrold.exe`; it starts the receiver in a hidden child process and keeps it alive in the tray. The app and agent must be adjacent. The receiver publishes `_seamlesscontrol._tcp.local.` with protocol version and public fingerprint through `mdns-sd`, while the app browses that service. Discovery is a hint only: the existing Noise XX key pinning and six digit pairing approval remain mandatory. On Windows, the app's firewall buttons launch the agent elevated to add inbound TCP rules for the selected control or file port and a separate UDP 5353 rule for mDNS. These rules are limited to the Private profile and LocalSubnet. The buttons report an authorization request; the elevated command reports whether the rule was applied. The GUI is built in Windows CI with a static C runtime. The release workflow builds the tagged Windows commit and attaches one ZIP containing both executables, with its adjacent SHA-256 file, to the GitHub Release. The hidden child uses a hidden clipboard owner window, so text clipboard writes do not depend on an attached console.

This guide covers manual operation, packaging, security, and current limits. The addresses below are **fictional examples**: source `192.168.50.10`, receiver `192.168.50.20`, LAN `192.168.50.0/24`, interface `wlan0`.

## Packaging and updates

`omarchy plugin add https://github.com/PuroDelphi/seamlesscontrol.git --enable` installs the Omarchy widget. Omarchy does not run repository install hooks. The widget's **Install agent** button opens an Omarchy terminal and runs `packaging/install-agent.sh`. This installs missing `rust`, `avahi` and `wl-clipboard` packages with `omarchy pkg add`, builds the release binary, and installs it in `~/.local/bin`. The installer records only packages it installed under `~/.local/state/seamlesscontrol/installed-packages`. Avahi is enabled for mDNS discovery when needed. Four user services are installed but left disabled. Existing service files are preserved if their contents differ from the plugin copies.

The terminal fallback is:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh
```

To update the widget on **each** machine, run `omarchy plugin update seamlesscontrol.control`, followed by `omarchy restart shell` so the panel loads the new QML. For agent code changes, stop any active session, choose **Update agent** in the panel (or use the terminal command), then restart the agent on both ends. An already running process keeps using its old binary until restarted. Pairing keys and layout are preserved.

**Remove agent** in the panel runs `uninstall-agent.sh --remove-deps` in an Omarchy terminal. It stops and removes unchanged plugin service files, preserves modified service files, removes the binary and removes only packages recorded as installed by this plugin. Pacman will refuse removals needed by other packages. Packages that predated the plugin are never recorded. The default CLI uninstall, without `--remove-deps`, leaves packages installed. Neither path deletes `~/.config/seamlesscontrol/` (identity, peers, layout). Remove a firewall rule separately before `omarchy plugin remove seamlesscontrol.control`.

## Manual connection

Start the receiver, then discover and pair from the mouse computer:

```bash
seamlesscontrold serve-auto 47832
seamlesscontrold local-address 47832
seamlesscontrold discover
seamlesscontrold pair 192.168.50.20:47832
```

The first command runs on the receiver; the last two run on the source. `serve-auto` selects an IPv4 address using the mDNS route and advertises `_seamlesscontrol._tcp` through Avahi. Discovery requires multicast mDNS; use `seamlesscontrold serve 192.168.50.20:47832` and a manual IP if multicast is blocked. An mDNS fingerprint is only a hint. Pairing uses Noise XX and a six digit comparison code; approve **on both computers** in the panel or with `seamlesscontrold status` followed by `seamlesscontrold approve <six-digit-code>`. `status` shows `serve pairing` on the receiver while approval is pending. `seamlesscontrold reject` cancels it. The 64 character local identity is a persistent public fingerprint, not the comparison code.

Place each computer next to the other on both layouts. On the source, for a receiver to the right:

```bash
seamlesscontrold topology set 192.168.50.20 1 0
seamlesscontrold connect 192.168.50.20:47832
```

On the receiver, place itself on the right and the source on the left:

```bash
seamlesscontrold topology set local 1 0
seamlesscontrold topology set 192.168.50.10 0 0
```

`connect` infers the edge from adjacent layout cells; an explicit `right`, `left`, `top`, or `bottom` argument overrides it. Return by crossing the receiver edge toward the source, pressing Escape on the physical keyboard, or running `seamlesscontrold return` on the receiver. `seamlesscontrold emergency-stop` on the receiver disconnects and pauses new input until `seamlesscontrold resume`. On the source, `pause` and `resume` toggle capture. Stop a terminal agent with Ctrl+C.

Other useful commands: `seamlesscontrold peers`, `topology`, `topology route <IP>`, `revoke <IP>`, `diagnose`, and `latency <IP:port>`. `diagnose` reports cursor, monitor rectangles and `LOCK unlocked|locked|undetermined`; run it in the graphical session. `latency` reports encrypted round trip min/p50/p95/max, excluding capture and display delay. `rotate-key` requires a stopped agent and forces remote machines to pair again.

## Firewall

The receiver's TCP port must be reachable **from the LAN only**. The panel previews the exact detected interface, subnet, receiver IP and selected port before requesting system authorization. Its script supports `show`, `allow` and `remove` with a port argument:

```bash
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh show 47832
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh allow 47832
bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/firewall-lan.sh remove 47832
```

`allow` and `remove` ask for confirmation, then use Polkit in a graphical session or sudo in a terminal. The panel has already shown the rule and confirmed the click, so it passes `--yes`; system authorization is still required. The script does not enable UFW. If the receiver address or subnet later changes, inspect `sudo ufw status numbered` and remove stale rules manually. File receiving normally uses TCP `47833` and needs a separate scoped rule if blocked.

## Protocol and other features

The source captures input through the desktop portal and EIS. The receiver injects it through Hyprland virtual input. Its virtual keyboard uses the compositor's XKB keymap to update Wayland modifier state with each key event, so combinations such as Super+V can reach Hyprland as shortcuts. Network sessions use Noise XX with pinned peer keys. The receiver permits one input owner at a time and releases held keys/buttons on disconnect. Lock state is checked through Hyprland IPC; unknown or locked state blocks injection. After Escape or a remote return, a 96 pixel rearm distance keeps the edge from capturing immediately again.

Text clipboard updates are UTF-8, limited to 256 KiB, and exclude sensitive Wayland selections. The experimental 2×2 mesh mode uses `seamlesscontrold mesh 47832` on the source after all receivers are paired and placed on every layout. It authenticates each destination and releases one input owner before switching to the next. Revoking a paired identity closes its retained mesh connection regardless of input ownership; queued feedback from that connection is discarded, and revoking the current owner releases capture. Mesh clipboard, rapid handoffs and network recovery still need physical multi-machine validation.

Files use a separate Noise session and receiver approval. In the panel, start **Wait for a file** on the receiver, choose a paired destination and a file on the source, then approve the offer on the receiver. The file port (`47833` by default) needs a separate LAN firewall rule from the control port; the panel previews and authorizes it. The panel launches `seamlesscontrold choose-file en|es` or `choose-folder en|es` as a separate process; these commands ask the existing desktop portal to choose a local path and print only that path as a JSON string. They do not open a file or begin a transfer. CLI transfer equivalents are `receive-file-auto 47833 ~/Downloads` and `send-file 192.168.50.20:47833 /path/to/file`. The UI persists a per-computer limit in MiB (1–10,240; default 100), passes `SEAMLESSCONTROL_MAX_FILE_BYTES` to new file-transfer processes, and enforces it on both sender and receiver. An already running file receiver must be restarted after a change. The CLI still accepts the environment variable directly. Size and SHA-256 are verified before publication. Files are never overwritten. One physical file transfer between two Omarchy machines passed on 2026-10-02; see [test results](TEST-RESULTS.md).

Copied-file transfers reuse the authenticated Noise file session on TCP `47834`, leaving the one-shot manual receiver on `47833`. The Windows app and Omarchy bar widget run a clipboard-file receiver while open. Windows reads one local regular file from `CF_HDROP`. Omarchy reads `text/uri-list` and the `copy`/`cut` payloads of `x-special/gnome-copied-files`, `x-special/mate-copied-files`, and `x-special/nautilus-clipboard`; KDE's `application/x-kde-cutselection` marker prevents a cut from being treated as a copy. GNOME's Recent view can put a `recent://` reference in the URI list while its GNOME payload contains the actual local `file://` URI. When needed, GIO may resolve one `recent://`, `starred://`, `search://`, or `trash://` reference to a local `file://` target. The result still has to be one regular local file within the size limit; symlinks, directories, remote targets, and multiple files are ignored. These formats describe compatibility, not a physical validation of every file manager. [GIO file attributes](https://docs.gtk.org/gio/file-attributes.html) document `standard::target-uri`; [CopyQ's format reference](https://github.com/hluk/CopyQ/blob/master/docs/faq.rst) describes common file-manager clipboard MIME types.

If exactly one peer is paired, the source offers the file automatically; otherwise the source UI asks for a destination. The receiver gets a topmost Windows approval dialog or an actionable Omarchy desktop notification, plus buttons in its Files view. Rejection prevents content transfer. On approval, the existing file receiver checks size and SHA-256, publishes into a unique private staging directory, and only then places a local `CF_HDROP` or URI-list reference on the receiver clipboard. The user pastes in a file manager. Staging keeps separate directories for repeated names, has a quota of at least 1 GiB or four times the configured per-file limit, and removes sessions older than seven days. The receiver needs a separate private-LAN TCP `47834` rule. The text clipboard adapter ignores file-list selections to avoid replacing them with text. The current implementation supports one regular file at a time; native folder-specific paste timing and virtual Shell files are outside its scope. Physical approval and paste succeeded in both cross-platform directions; same-OS pairs still need physical checks.

User services installed but not enabled: `seamlesscontrol-receiver-auto.service`, `seamlesscontrol-receiver.service`, `seamlesscontrol-sender.service`, and `seamlesscontrol-mesh.service`. The auto receiver uses `47832`. Manual service environment files in `~/.config/seamlesscontrol/` can set `SEAMLESSCONTROL_LISTEN`, `SEAMLESSCONTROL_PEER`, `SEAMLESSCONTROL_EDGE`, or `SEAMLESSCONTROL_PORT`. Enable only the chosen service per machine.

## Verification and limits

### Observed connection stall on two Omarchy machines

During a panel-started connection on 2026-10-02, the receiver reported `serve connected` and an established TCP connection. The source reported `connect connecting` with the receiver already identified for several minutes. The input-capture portal had no new session events for that attempt. After ending the stuck source process and restarting the source's `xdg-desktop-portal-hyprland` user service, a fresh connection created a portal session, obtained zones, installed the edge barrier, connected to EIS, and reached working pointer capture. This supports a stalled desktop capture setup after network authentication; the exact call that hung is not proven. A prior monitor layout change may be relevant but is not proven causal.

The panel now distinguishes network retry, authenticated capture setup and ready-to-cross states. After 15 seconds in authenticated capture setup, a UI action stops the panel-started source process, restarts only the source's Hyprland portal service, and asks the user to select Connect again. It warns that other screen-sharing apps on that source may be interrupted. The panel's normal Stop action falls back from SIGINT to SIGTERM after two seconds if the agent is stuck before entering its main loop. The receiver keeps listening. This recovery was based on the observed incident; the new button still needs a live stall test.

```bash
bash tests/firewall_lan.sh
bash tests/agent_setup_packages.sh
bash tests/uninstall_agent.sh
cargo test --manifest-path agent/Cargo.toml
cargo clippy --manifest-path agent/Cargo.toml --all-targets -- -D warnings
```

Loopback pairing, discovery, file, reconnect and roaming scripts are listed in [TESTING.md](TESTING.md). Integrated capture and injection tests require a real Omarchy session without another active agent. Two physical Omarchy machines confirmed edge return, Escape, Super+V and a file transfer. The [Windows x64 app](WINDOWS.md) shares the wire protocol. Physical Windows 11 x64 tests confirmed control in both directions, return via screen edge and Escape, text clipboard in both directions, the tested keyboard shortcuts, an Omarchy→Windows file transfer and automatic Windows discovery in the Omarchy panel. Larger transfers, multi-machine mesh, different monitor layouts, sleep and network loss remain additional test scenarios; the [test record](TEST-RESULTS.md) states their scope.
