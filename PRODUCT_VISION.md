# UniKey — Product Vision

## The Fundamental Insight

The problem with every cross-device tool on the market is the same: **they make you think about the tool.**

You open an app. You see a dashboard. You check connection status. You select a device. You click "send." You wait. You see a confirmation.

Each step is a tax on attention.

UniKey exists to remove that tax entirely.

**UniKey is not an app you "use." It's a layer that "works."**

If you have to open UniKey, UniKey has failed.

---

## What Should Disappear?

| Current assumption | What disappears |
|---|---|
| Open app to send clipboard | Clipboard should already be there |
| Check connection status | If it works, you don't need to know |
| Navigate to a screen | The entire concept of navigation |
| Find a device in a list | Devices should be reachable by proximity |
| Click "pair" | Pairing starts when devices are near |
| Choose settings | Settings should be contextual or zero |
| See a loading state | 16ms or it failed |
| Read instructions | The interface explains itself |
| Open a settings panel | App-level settings: 3 yes/no decisions max |
| Close the app | The app should never need opening |

---

## The Redesigned Application

### Physical Form

Not a window. A **menu bar item** (macOS) / **system tray icon** (Windows).

Click it once → A compact floating panel appears (360×480).
Click outside → Panel disappears. No close button needed.

The panel is the entire application. There are no screens to navigate to.

### Panel Layout (when opened)

```
┌─────────────────────────────┐
│ 🔑 My MacBook Pro    ● live │
│ 💻 Windows Desktop   ● live │
│ 📱 iPhone 16         ○ away │
│ ─────────────────────────── │
│ + Pair New Device           │
│                             │
│  ─── Recent Clipboard ───  │
│  link: github.com/...       │
│  "const API_URL = ..."     │
│  Screenshot_2025-07-29.png │
└─────────────────────────────┘
```

That's it. This is the entire app.

**What's not here:**
- No sidebar
- No navigation tabs
- No settings gear icon
- No status indicators that don't communicate something actionable
- No "home" screen
- No dashboard
- No scrolling banners

### The Clipboard Is Everywhere

The most important feature of UniKey is the clipboard. It should feel like magic.

When you copy text on your MacBook and switch to your Windows PC, the clipboard should already contain what you copied. It should feel like the operating system's clipboard became omniscient. There should be no indication that UniKey was involved.

**The only visible UI for clipboard:** A global shortcut (Cmd+Shift+V) that opens a clipboard timeline overlay — just a thin floating list of recent clips. It appears. You pick one. It disappears.

### Pairing

No codes to type. No complex flows.

Devices on the same network see each other instantly. A notification appears: "Windows Desktop wants to pair. Approve?" Yes/No.

If you want to initiate: Open the panel → click "Pair New Device" → a QR code appears → scan with another device → done.

The pairing screen is just the core panel with a QR code in place of the device list. There's no "navigate to pairing screen." The panel just changes content.

### Device Actions

Right-click any device in the list. A contextual menu appears:

- Share File…
- Rename
- Disconnect
- Forget Device

No settings panel. No separate screen. Just right-click → action → done.

### Settings

There are exactly three settings, and they live in the bottom of the main panel as small toggles:

- [ ] Clipboard Sync
- [x] Keyboard Forwarding
- [x] Mouse/Trackpad

That's it. Everything else is inferred, automatic, or contextual.

---

## The Redesigned Website

Not a landing page. An **interactive product demonstration**.

### Structure

```
┌─────────────────────────────────────────┐
│  [logo]                          [nav]  │
│                                         │
│  ┌─────────────────────────────────┐   │
│  │                                 │   │
│  │   LIVE DEMO: Clips moving       │   │
│  │   between devices in real-time  │   │
│  │                                 │   │
│  │   Copy here → appears there →   │   │
│  │   [Type something…]             │   │
│  │                                 │   │
│  └─────────────────────────────────┘   │
│                                         │
│  "The clipboard that follows you."      │
│  One sentence. No subheadline.          │
│                                         │
│  [Download for macOS]                   │
│                                         │
│  ─── scroll ───                         │
│                                         │
│  Three panels. Each one sentence.       │
│  Each one demonstrates a single idea.   │
│                                         │
│  No feature list.                       │
│  No bullet points.                      │
│  No comparison table.                   │
│                                         │
│  ─── scroll ───                         │
│                                         │
│  Pricing: Free. That's it.              │
│                                         │
│  FAQ: Three questions. Max.             │
└─────────────────────────────────────────┘
```

### Core Principle

A visitor should understand UniKey before reading a single word.

The hero shows the product working. Text exists only to reinforce what the visitor already understood visually.

---

## Design Language — "Presence"

The visual system is called **Presence**:
- Design that recedes rather than announces
- UI that feels like a property of the system, not an application
- Materials that feel physical without being skeuomorphic

### Color

- **Base:** `#0A0A0B` — almost black. Not purely black. Has depth.
- **Surface:** `#141416` — slightly lighter, like light hitting a dark surface.
- **Accent:** `#7C3AED` — violet. Used only for interactive elements and only when necessary.
- **Connection:** `#30D158` — green. Used only for the live indicator.
- **Mono:** Everything else is shades of white between 6% and 80% opacity.

No blue links. No red errors (errors are communicated through absence, not alerts).

### Typography

One typeface: **Inter**.

One weight variation: **400** for body, **600** for emphasis. That's it.

No display weights, no thin weights, no compressed widths.

Font size hierarchy:
- 13px — captions
- 14px — body
- 15px — device names (bold)
- 56px+ — website hero (the only place for display type)

### Space

The app panel uses an 8px grid. Content padding: 20px. No exceptions.

Items are separated by space, not lines. If you need a divider, your layout is wrong.

### Motion

Everything uses a single spring:
- Tension: 280
- Friction: 26
- Mass: 1

This spring governs every animation. The result: consistent, physical, predictable motion.

Motion has three purposes only:
1. **Context** — showing where something came from (element morphing)
2. **Feedback** — confirming an action (subtle bounce)
3. **Focus** — directing attention (element appearing)

No decorative animations. No "delight" for its own sake. No parallax.

### Glassmorphism

Use only where it communicates depth:
- The main panel (it floats above the desktop)
- The clipboard overlay (it floats above other content)

Don't use glass for cards, buttons, or decorative surfaces. Those should be solid.

### Icons

Don't use an icon library. Icons are 16px, 1.5px stroke, custom for UniKey.

Every icon must pass the "silhouette test" — recognizable in outline form at 16×16.

---

## Motion Language — Specifications

### Panel Launch

```
Event: User clicks menubar icon
Animation:
  - Panel scales from menubar icon position
  - Duration: 200ms
  - Spring: tension 300, friction 28
  - Backdrop blur fades in: 80ms
```

### Device List → Entry

```
Event: New device discovered
Animation:
  - Row slides in from top
  - Duration: 300ms
  - Spring: tension 200, friction 22
```

### Clipboard Timeline

```
Event: User presses Cmd+Shift+V
Animation:
  - Overlay fades in (opacity 0→1)
  - Items stagger in from bottom
  - Duration: 150ms
  - Spring: tension 300, friction 30
```

### Device → Context Menu

```
Event: User right-clicks device row
Animation:
  - Menu appears at cursor position with subtle scale
  - Duration: 100ms
  - No spring (must feel immediate)
```

---

## Engineering Architecture — macOS App

### Process Model

```
┌─────────────────────────────────────────┐
│            System (menu bar)            │
│  ┌───────────────────────────────────┐  │
│  │  UniKeyAgent (daemon)            │  │
│  │  - Network discovery             │  │
│  │  - Clipboard monitoring          │  │
│  │  - Input forwarding              │  │
│  │  - Encryption                    │  │
│  │  - No UI                         │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  UniKey (SwiftUI/AppKit)         │  │
│  │  - Menu bar icon                 │  │
│  │  - Floating panel (popover)      │  │
│  │  - Clipboard overlay             │  │
│  │  - UI only                       │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

Two processes:
1. **Daemon** — runs in background, manages all state, no UI
2. **UI** — lightweight panel that reads daemon state, opens on demand

The daemon starts at login. The UI starts on demand (click menubar icon). This means:
- Daemon uses close to zero resources when idle
- UI process is cheap to launch and destroy
- No persistent app window consuming screen real estate

### Data Flow

```
Clipboard Event → Daemon detects → Encrypts → Sends to paired devices
                                         ↓
                              Receiving daemon decrypts
                                         ↓
                              Writes to local clipboard
                                         ↓
                              UI reads clipboard history from daemon
                                         (via IPC, on demand)
```

### State Management

Zero persistent state in the UI. The UI is a thin client. All state lives in the daemon and is queried on panel open.

This means:
- The UI never needs to "sync"
- Opening the panel is always instant
- The UI can crash and restart without losing anything
- The daemon manages pairing, trust, encryption keys

---

## Application Architecture — Current Flutter App

The current Flutter app is already architecturally aligned:
- `device_provider.dart` acts as the daemon proxy
- `api_client.dart` communicates with the Rust daemon via HTTP
- UI screens consume provider state

The changes needed are primarily **layout and interaction**, not architecture:

1. Remove the full-window shell (AppShell, sidebar, header)
2. Create a compact floating panel layout
3. Add menubar/tray integration
4. Replace navigation with contextual content switching
5. Add clipboard overlay

---

## The Implementation Path

### Phase 1 — Strip to Essence (Desktop App)
- [ ] Replace AppShell with a compact floating panel (360×480)
- [ ] Remove sidebar navigation; replace with device list
- [ ] Remove header; inline the content
- [ ] Remove detail panel; use contextual actions
- [ ] Move settings to inline toggles at panel bottom
- [ ] Integrate clipboard history (Cmd+Shift+V overlay)
- [ ] Add menubar icon (macOS)
- [ ] Add system tray icon (Windows)

### Phase 2 — Interaction Refinement
- [ ] Spring-based motion system
- [ ] Device list → contextual actions
- [ ] Pairing inline (no separate screen)
- [ ] Clipboard timeline overlay
- [ ] Reduced motion support

### Phase 3 — Website
- [ ] Live product demonstration in hero
- [ ] Strip brochure content to essence
- [ ] Remove feature grid, comparison tables
- [ ] Single CTA: Download
- [ ] Pricing: Free (remove tiers for initial launch)

---

## Decision Log

### Why no dashboard?
A dashboard communicates "check here for status." The ideal state is: nothing to check. If everything works, the list shows all devices green. If something needs attention, the menubar icon changes.

### Why no settings screen?
Three toggles fit at the bottom of the panel. Everything else is automatic or contextual. "Settings" screens imply complexity. Hiding them forces the system to be simple.

### Why no feature descriptions?
In the app, features should be self-evident. In the website, the live demo shows everything. Descriptions add cognitive load without value.

### Why no animations in the app?
Animations in the app serve a purpose: feedback, context, focus. Decorative animations (scanning radar, particle effects) erode trust in a productivity tool.

### Why a single pricing tier?
Pricing tiers imply different levels of capability. For a product that should disappear, there is only one level of capability: it works. Free during beta. Paid later, but one price.

---

## Future Direction — Post-Invisibility

After the product achieves invisibility — meaning users forget it exists — these expansions become natural:

- File transfer (drag from device to device)
- Universal keyboard (type on any device)
- Universal mouse (cursor crosses screens)
- Universal media controls

Each one inherits the same philosophy: it should feel like the OS grew a capability, not like you installed another app.
