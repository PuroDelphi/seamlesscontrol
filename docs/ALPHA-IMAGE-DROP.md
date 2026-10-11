# Image clipboard and file drop · alpha validation

This document tracks the remaining physical checks before these changes can be considered for `main`.

## Implemented

- [x] Add bounded PNG clipboard messages to the authenticated control channel, including mesh forwarding and loop prevention.
- [x] Read SVG and supported Wayland raster MIME types and Windows registered raster formats or supported DIB/DIBV5 bitmap selections; convert non-PNG formats to bounded PNG. Publish PNG on Omarchy and PNG plus DIBV5 on Windows.
- [x] Keep image clipboard under the existing per-peer Clipboard permission and active control session. Ignore unsupported or oversized images instead of breaking mouse or keyboard control.
- [x] Let users drop local files and folders onto the sender's Files page on Omarchy or Windows. Reuse the existing authenticated file offer, Files permission, receiver approval mode, size limit, hash verification and staging. Never move or delete the source.
- [x] Update English and Spanish user guides with the new workflow and its limits.

## Verify on physical computers

### Two Omarchy computers · 2026-10-11

- [x] Both computers ran plugin 0.25.2 (`2030496`) and agent protocol 7 during an authenticated control connection.
- [x] A 3×2 JPEG and 4×3 SVG copied on the other Omarchy arrived here as PNG with the expected dimensions and pixel colors.
- [x] A 5×4 WebP copied here arrived there as PNG with the expected dimensions and pixel colors. Its tiny size made it look blank in the target app; a 640×360 WebP then pasted visibly with its green gradient and dark border.
- [x] A user copied an image from an ordinary application on the other Omarchy and pasted it successfully in an application here.

### Remaining checks

- [ ] Omarchy → Windows: copy a screenshot/image from an application and paste its pixels into an image editor or chat composer. Verify text still syncs after the image.
- [ ] Windows → Omarchy: test a Snipping Tool image and browser copies of JPEG, GIF, WebP and SVG images. Paste pixels into an application. Verify transparency where the source has it.
- [ ] With Clipboard permission off, confirm image and text do not arrive. With no active control connection, confirm neither arrives.
- [ ] Drop a file and a folder onto the sender's Files page in each OS direction. Confirm one receiver offer, Ask every time / automatic / timed approval behavior, size limit, rejection, no transfer after session stop, and original remains in place.
- [ ] On Windows, confirm that dragging a paired computer onto the screen map still works after enabling native file drops in the app.
- [ ] Test an image above the 32 MiB converted-image clipboard limit and an unsupported drag source. Control must remain responsive and the UI must explain how to send the content as a file.

## Separate native drag bridge

Dropping on the sender's app is implemented. A continuous drag from a file manager across the screen edge into the other OS's file manager needs a native data-object bridge. Wayland sends drag offers to a destination surface, while Windows uses OLE `IDataObject`/`IDropTarget`; the existing EIS/Windows pointer capture transports input only. Implement this only after a design that preserves the Files permission, receiver approval, size and symlink checks, cancellation, and copy-only behavior. Do not present an unfinished cross-desktop drag as an active feature.
