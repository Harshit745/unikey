# Screen-edge file handoff

UniKey's current experimental edge flow lets a user drag files from any Finder
or Windows Explorer location to a narrow, always-on-top strip on a configured
left or right screen edge. Releasing on that strip submits the real local file
paths to the existing encrypted, paired-device transfer service.

## Enable it

1. Pair the two devices. A discovered or connected-but-untrusted peer is not a
   valid file destination.
2. Keep both daemons online on the same LAN.
3. Open **Files**, choose the paired target, select the shared left or right
   edge, and choose **Enable edge**.
4. Drag one or more files to the blue edge strip and release.
5. Treat the transfer as successful only when its status becomes `Verified`.

The receiving daemon writes verified content into its configured UniKey
download directory. By default this is inside the per-user UniKey data folder.

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
