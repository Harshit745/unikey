# Screen-edge file handoff

UniKey's current experimental edge flow lets a user drag files from any Finder
or Windows Explorer location to a narrow, always-on-top strip on a configured
left or right screen edge. Reaching that edge submits the real local file paths
to the existing encrypted, paired-device transfer service; the UniKey window
does not need to be open or focused.

## Enable it

1. Pair the two devices. A discovered or connected-but-untrusted peer is not a
   valid file destination.
2. Keep both daemons online on the same LAN.
3. Open **Files**, choose the paired target, select the shared left or right
   edge, and choose **Enable edge**.
4. Drag one or more files to the blue edge strip. Reaching the strip initiates
   the handoff; release there to finish the local operating-system gesture.
5. Treat the transfer as successful only when its status becomes `Verified`.

The edge stays enabled while navigating between UniKey pages or hiding its main
window. Return to Files and choose **Disable edge** to stop it. It is off after
the desktop app quits. The strip leaves native title-bar controls and the
Dock/taskbar clear and does not take keyboard focus.

The secondary window has a separate, event-listening-only Tauri capability;
without it native drag callbacks are rejected. Listener failures and transfer
errors are now visible in the strip. Blue means pending; green means the remote
receiver has actually verified every file, not merely accepted an API request.
The target device cannot be changed while the current edge is enabled.

The receiving daemon writes verified content into its configured UniKey
download directory. By default this is inside the per-user UniKey data folder.

## Verified checkpoint (2026-10-24)

Source build `3439c82` was installed on the Mac. After refreshing the approved
app and helper Accessibility grants, native cursor capture started successfully;
cursor receiving was then turned off. The right-edge file target remained
enabled after navigating from Files to Input and back.

The user dragged a generated, non-personal 173-byte text file from Finder to the
Mac's right-edge strip and confirmed the interaction worked. The live transfer
API and Files screen reported `Verified` after the paired Dell's acknowledgement.
This verifies Mac-to-Windows edge handoff, not Windows-to-Mac native dragging or
continuous dragging into arbitrary remote applications.

The [Windows installer build and lifecycle checks](https://github.com/Harshit745/unikey/actions/runs/40085960363)
passed. [The matching Windows artifact](https://github.com/Harshit745/unikey/actions/runs/40085960363/artifacts/12556694885)
contains `UniKey-Setup.exe` and its independently checked SHA-256 file:
`dadb8778b50a9269c24541bb09ad0c471a2348abfbaee8fb28500fdab9d9805c`.
This is an expiring CI artifact, not a published GitHub release.

## Current boundary

This milestone is an edge **handoff**, not yet a continuous native drag session
over arbitrary applications on the receiving computer. Continuing the same
held mouse gesture into Finder, Explorer, Mail, or another app requires two
platform bridges:

- Windows OLE `IDataObject`/`IDropSource`/`IDropTarget` handoff.
- macOS AppKit `NSFilePromiseProvider`/`NSDraggingSession` handoff.

Those bridges must preserve the operating system's drag lifecycle, stream the
file as a promise, and materialize it only after receiver verification. Until
they are implemented and physically tested in both directions, UniKey must not
describe the feature as seamless cross-OS drag-and-drop.
