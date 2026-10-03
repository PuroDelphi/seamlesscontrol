# Plan: copy a file here, paste it on another computer

Status: design only. No file clipboard integration is implemented yet.

## User experience

1. Copy one regular file in the file manager on a paired Omarchy or Windows computer. SeamlessControl shows its name and size to the intended paired computer, without transferring its contents yet. If several peers are available, choose the destination in the app.
2. On the destination, select **Receive copied file** from a notification or the SeamlessControl panel. The offer shows the sender, name, size, and destination folder. The user can decline it.
3. After approval, the existing encrypted file-transfer protocol downloads the file to a private staging folder, verifies size and SHA-256, and never overwrites an existing file. SeamlessControl then places the verified local file reference on the destination clipboard. The user presses Paste in Explorer or the Omarchy file manager to put a copy in the chosen folder.
4. Show transfer progress, completion, rejection, and errors in both UIs. Keep the current explicit **Send file** workflow available.

## Engineering steps

1. Add a versioned, typed clipboard event (`text` or `file offer`) to the current clipboard channel. Preserve text sync and suppress echo loops. An offer contains a random transfer ID, peer identity, safe display name, size, and expiry; it does not expose a usable local path to the other computer. Older agents must keep text sync working.
2. Observe file clipboard formats: Windows Shell `CF_HDROP`; on Omarchy, the Wayland clipboard MIME types offered by the active file manager, beginning with `text/uri-list`. Read and validate one local regular file. Reject remote or non-file URIs, symlinks, directories, unsupported virtual files, and paths outside the local file system in the first release. Test actual Omarchy file managers before selecting extra MIME types.
3. Add the recipient prompt and peer choice to both UIs, with a background file-offer listener so the receiver does not need to press **Wait for a file** first. Explain the separate private-LAN file-port firewall rule. Reuse paired identities, local approval, the per-computer size limits, and the existing Noise file stream. A copied file never starts a transfer on its own. Cancel an offer if the source clipboard changes, the source file changes, the peer disconnects, or a short expiry passes.
4. Stage an accepted file in a private, quota-controlled folder. Publish a local file clipboard reference only after verification: `CF_HDROP` on Windows and the file manager's supported URI-list format on Omarchy. Keep the staged file available while the clipboard refers to it; define cleanup and a user-visible recovery path if Paste is never used.
5. Test Omarchy ↔ Omarchy, Omarchy ↔ Windows, and Windows ↔ Windows: both directions, Unicode names, duplicate names, cancellation, interrupted transfer, changed source, oversize file, insufficient disk space, clipboard replacement, and text clipboard regression.

## Feasibility boundary

The first version can deliver the familiar **Copy → approve → Paste** flow. Detecting a paste into an arbitrary folder before approval is application-specific and should not be assumed: Windows Shell and Wayland file managers expose file references through different clipboard formats. The acceptance panel provides the safe trigger; after verification the normal file manager Paste action performs the final placement. Explore delayed-rendered Windows clipboard objects and Wayland data offers later only if real tests show a clear improvement.

References: [Windows Shell clipboard formats](https://learn.microsoft.com/en-us/windows/win32/shell/clipboard), [Wayland data-control protocol](https://wayland.app/protocols/ext-data-control-v1), [RFC 2483 URI list](https://www.rfc-editor.org/rfc/rfc2483.html).
