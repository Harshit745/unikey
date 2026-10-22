# UniKey — Design Language v2

## Philosophy

Most software is designed to be looked at.
UniKey is designed to be felt.

When you type on your MacBook and paste on your Windows PC, the system should feel like it grew a capability — not like you installed an application.

The highest praise UniKey can receive: "I forgot it was there."

## What UniKey Is Not

UniKey is not a dashboard.
It is not a clipboard manager.
It is not a device manager.
It is not a file transfer app.
It is not KVM software.

These are all boxes that software companies build around their products to sell them.
UniKey is not a product you buy.
It is a layer you inhabit.

## The Interface Should Feel Like

- A property of the operating system
- A natural extension of the cursor, clipboard, and keyboard
- Something that has always been there
- Invisible until needed, effortless when used
- Calm, never demanding attention

## Mental Model

Think of Dynamic Island, Control Center, Spotlight, AirDrop, AirPlay.

These are not applications.
They are **capabilities** of the system.

UniKey should feel the same way.

---

## Visual Language

### Color

The UI is almost entirely monochromatic.
Color is reserved for function.

Surface: pure white in light mode / #121212 in dark mode
Glass: RGBA white at 3-8% opacity with blur
Text: pure black / pure white
Functional colors:
  - Connection: soft green, muted, never bright
  - Active feature: subtle blue-white glow
  - The app should feel like it's made of titanium and glass — not neon and plastic.

No purple. No cyan. No emerald (as branding colors).
No gradients that serve no purpose.
No colored cards, buttons, or borders.

The ONE exception: the connection eye icon. A tiny blue-white pulse when devices are synced.

### Typography

The UI uses the system font (SF Pro on macOS).
No Inter, no custom fonts.

Type scale:
  - 11px / 400 — captions, timestamps
  - 13px / 400 — body, clipboard content
  - 14px / 500 — device names
  - 15px / 600 — section emphasis (rare)
  - Huge editorial type (48px+): reserved exclusively for the website hero

Typography IS the interface.
No labels on icons.
No decorative display type.
Everything is set in a single weight (500 for hierarchy, 400 for everything else).

### Spacing

8px grid. Content uses 16px gutters. 24px for breathing room.
Items are separated by space — never by lines or borders.

Border exists in exactly ONE place: the edge of the menu bar dropdown.
Inside the dropdown, everything is separated by vertical space.
If you need a divider, your spacing is wrong.

### Glass

Glass is real. It is not a visual effect.

Glass has:
- Translucency (light passes through)
- Blur (what's behind is visible but diffused)
- A faint tint (not colored — just slightly lighter/darker than background)
- No border at the edge of content
- An inner shadow when it overlaps another surface

Use glass for:
- The menu bar dropdown (it overlays the desktop)
- The clipboard overlay (it overlays content)
- Pairing sheet (temporary, dismissable)

Do NOT use glass for:
- Buttons
- Cards
- Device icons
- Any element that should feel solid

### Depth

Depth is created through blur layering, not shadows.

Layer 0 — Desktop (no blur)
Layer 1 — Menu bar dropdown (subtle blur, sigma 8)
Layer 2 — Clipboard overlay (medium blur, sigma 12)
Layer 3 — Pairing sheet (heavier blur, sigma 16)

Each layer is slightly higher blur than the previous.
Each layer has a slightly lighter/darker tint.
Nothing has a drop shadow.

The effect: the UI feels like it exists in physical space, with the desktop naturally receding behind each layer.

### Motion

Spring physics everywhere.
Single spring configuration:
  - tension: 250
  - damping: 25
  - mass: 1

This spring is used for:
  - Dropdown appearing/disappearing
  - Device list items appearing
  - Clipboard items fading in
  - Status dot transitions
  - Cursor following (for mouse sharing visualization)

Motion has three purposes:
  1. **Context** — showing spatial relationships (where a clip came from)
  2. **Continuity** — maintaining user orientation (device appeared, didn't pop in)
  3. **Calm** — avoiding startling the user (everything is gradual)

No decorative animations.
No particle effects.
No scanning radars.
No animated backgrounds.

### Device Representation

Devices are represented by their NAMES, not icons.
The name is the interface.

When a device needs a visual indicator:
  - A tiny 4px dot next to the name (green = connected, gray = away)
  - The dot has a subtle pulse animation when the device is actively sharing

No platform icons in the device list.
No need to show a laptop icon for a laptop.
The name "MacBook Pro" is sufficient.

The one place platform icons appear: the pairing screen, to help identify unfamiliar devices.

---

## Interaction Model

### Menu Bar Dropdown

This is the primary interface.

Trigger: Click menubar icon
Appearance: Sheet slides down from menubar, attached at top edge
Dismiss: Click outside, press Escape, or click menubar icon again
Position: Below menubar, right-aligned to icon
Size: 280px wide, height determined by content
Material: Glass (sigma 8 blur), no border radius at top, 12px radius at bottom corners
Animation: Spring slide + subtle opacity (150ms)

Content (top to bottom):
  1. Current device name (quiet, secondary treatment)
  2. Paired devices (list of names with status dots)
  3. Recent clipboard (bare text, last 3-5 items, timestamp)
  4. Feature pills (inline: Clipboard · Keyboard · Mouse — tiny dots indicating on/off)
  5. Pair new device (quiet text action at the very bottom)

No section headers.
No "Devices" label.
No "Clipboard" label.
Everything is self-evident from its position and treatment.

### Clipboard Overlay

Secondary interface, invoked by keyboard shortcut (Cmd+Shift+V).

Appearance: A thin strip, 240px wide, positioned near the cursor or bottom-right of active monitor
Content: Last 10 clipboard items, pure text, chronologically
Material: Glass (sigma 12), rounded 12px corners
Animation: Spring slide from the edge, items stagger with slight delay

Dismiss: Click outside, press Escape, or select an item
Selection: Click an item — it's copied to clipboard and overlay closes instantly

No search bar.
No pinning.
No icons.
No categories.
Just text, in order, beautifully spaced.

### Pairing

Not a screen. A sheet.

When you click "Pair New Device":
  1. The dropdown smoothly transforms — devices slide out, QR code slides in
  2. OR: a thin sheet slides up from the bottom of the screen

The sheet has:
  - A large QR code (the only large element in the app)
  - Device name below it
  - "Scan with UniKey on another device" — one line
  - Cancel link (not a button)

No instructions.
No code entry field.
No "waiting for connection" spinner.

When successfully paired:
  - Sheet dissolves
  - Menubar icon briefly pulses
  - A notification appears in the system notification center: "Paired with Windows Desktop"

### Trust Request

When an unknown device requests pairing:
A small glass banner slides in from the top of the screen:
  ┌────────────────────────────────────┐
  │ "Windows Desktop wants to pair."   │
  │            [Decline]  [Approve]    │
  └────────────────────────────────────┘

This is the only notification pattern in the entire application.
No dialogs. No modals. No pop-ups.

The banner auto-dismisses after 10 seconds if no action is taken.

### Settings

There are no settings.

The three features (Clipboard, Keyboard, Mouse) are toggled by tiny inline pills at the bottom of the dropdown.
They are the only interactive elements besides the device list.

Everything else is automatic:
  - Discovery happens on LAN
  - Pairing uses exchanged keys
  - Encryption is always on
  - Clipboard sync is on by default

If a user needs to forget a device: right-click the device name in the list → "Forget"
If a user needs to quit the app: right-click the menubar icon → "Quit UniKey"

That is the complete settings surface.

---

## Screen Flows

There is exactly ONE screen flow in UniKey:

1. User is working
2. User needs clipboard from another device
3. User presses Cmd+Shift+V
4. Clipboard overlay appears
5. User selects the clip
6. Overlay disappears
7. User pastes

Total interaction time: 2 seconds.
Zero navigation. Zero decisions. Zero load time.

The only other flows:
  - Pairing: Click "Pair" → QR appears → Scan → Done
  - Trust: Device approaches → Banner appears → Approve → Done
  - Check status: Glance at menubar icon → Green = working

---

## The Website

The website is not a landing page.
It is a demonstration.

Structure:
  - One page
  - Hero: interactive clipboard demo (type in one box, appears in another)
  - Three principles (one sentence each, large type, editorial spacing)
  - Pricing: "Free during beta" (centered, huge, understated)
  - FAQ: three questions
  - Footer: minimal

No feature grid.
No comparison table.
No "About" section.
No blog.
No changelog.

The hero is the entire product story.
Everything below is merely answering obvious questions.

Typography on the website is editorial: large, confident, beautiful.
Inter or system font. 48-72px headlines. 24px body text.
Massive whitespace. Almost no color.

---

## Implementation

### Desktop App

The existing Flutter app should be refactored to:

1. A single menu bar icon with dropdown (the flagship interface)
2. A clipboard overlay (triggered by shortcut)
3. A pairing sheet (transforms from dropdown or slides up)
4. A trust banner (system notification style)

Architecture:
  - The Rust daemon handles everything: discovery, clipboard, encryption
  - The Flutter UI is a thin client that reads daemon state via HTTP
  - The UI has NO persistent state — it opens, reads state, renders, closes
  - When the UI closes, the daemon continues running

### Website

The existing Next.js website should be refined to:

1. Clean, editorial typography
2. Live clipboard demo as the hero
3. Minimal content sections
4. No boxes, no cards, no borders

The current website direction is correct but needs:
  - Remove glass-card utility (no card borders anywhere)
  - Increase type sizes in hero
  - Reduce visual effects (no gradient orbs, no radial glows)
  - More whitespace
  - Remove any element that looks like a card or panel
