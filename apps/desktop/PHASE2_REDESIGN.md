# Phase 2 — Complete Redesign Document

> UniKey — by the Apple Universal Control, Handoff, and Arc Browser team
>
> "This feels like an Apple product."

---

## Table of Contents

1. [Design Philosophy](#1-design-philosophy)
2. [Color System](#2-color-system)
3. [Typography System](#3-typography-system)
4. [Spacing & Layout](#4-spacing--layout)
5. [Elevation & Shadows](#5-elevation--shadows)
6. [Radius & Shape Scale](#6-radius--shape-scale)
7. [Glass & Blur Effects](#7-glass--blur-effects)
8. [Iconography](#8-iconography)
9. [Motion System](#9-motion-system)
10. [Component Architecture](#10-component-architecture)
11. [Screen Specifications](#11-screen-specifications)
12. [State System](#12-state-system)
13. [Accessibility](#13-accessibility)
14. [File Structure](#14-file-structure)

---

## 1. Design Philosophy

### Principles

| Principle | Meaning |
|-----------|---------|
| **Deference** | UI never competes with content. Every element has a reason. |
| **Clarity** | Text is legible at every size. Icons are identifiable. Purpose is obvious. |
| **Depth** | Layers help establish hierarchy. Light, shadow, and blur communicate what's interactable. |
| **Calm** | No jarring transitions. No buzzing UI. Motion is purposeful and gentle. |
| **Progressive Disclosure** | Show essentials. Reveal details on demand. Never overwhelm. |
| **Direct Manipulation** | Cards lift on hover. Controls respond instantly. Physics feels real. |

### Personality

UniKey is **professional**, **minimal**, and **connected**. It should feel like a native macOS utility — System Settings meets AirDrop with the polish of a professional creative tool.

Words: *Precise. Elegant. Connected. Fast. Private.*

---

## 2. Color System

### 2.1 Core Palette

Colors are semantic and named by purpose, not by value.

```dart
// ─── Semantic Colors ─────────────────────────────────────────────

// Backgrounds
Color backgroundPrimary    = Color(0xFF0A0A0B);  // Main window bg
Color backgroundSecondary  = Color(0xFF0F0F12);  // Sidebar bg
Color backgroundTertiary   = Color(0xFF151519);  // Card surface bg

// Surfaces
Color surfacePrimary       = Color(0xFF1C1C20);  // Card default
Color surfaceSecondary     = Color(0xFF222227);  // Elevated surface
Color surfaceTertiary      = Color(0xFF2C2C30);  // Interactive hover

// Separators & Borders
Color separatorPrimary     = Color(0xFF1E1E22);  // Strong separator
Color separatorSecondary   = Color(0xFF2A2A2E);  // Subtle separator
Color separatorTertiary    = Color(0xFF38383C);  // Hover border

// Text
Color textPrimary          = Color(0xFFFFFFFF);  // Primary content
Color textSecondary        = Color(0xFFE5E5EA);  // Secondary content
Color textTertiary         = Color(0xFF8E8E93);  // Tertiary / placeholder
Color textQuaternary       = Color(0xFF636366);  // Disabled / metadata

// Accent — the UniKey brand color
Color accentPrimary        = Color(0xFF7C6CFF);  // Primary accent
Color accentSecondary      = Color(0xFF9B8FFF);  // Hovered accent
Color accentDim            = Color(0xFF3A3070);  // Accent on dark surfaces

// Semantic Status
Color statusOnline         = Color(0xFF30D158);  // Connected / online
Color statusOffline        = Color(0xFF8E8E93);  // Offline / away
Color statusBusy           = Color(0xFFFF9F0A);  // Busy / in-use
Color statusError          = Color(0xFFFF453A);  // Error / critical
Color statusWarning        = Color(0xFFFFD60A);  // Warning
Color statusSuccess        = Color(0xFF30D158);  // Success

// Fill colors (for containers, badges, pills)
Color greenFill            = Color(0xFF30D158).withValues(alpha: 0.12);
Color redFill              = Color(0xFFFF453A).withValues(alpha: 0.12);
Color orangeFill           = Color(0xFFFF9F0A).withValues(alpha: 0.12);
Color blueFill             = Color(0xFF0A84FF).withValues(alpha: 0.12);
Color purpleFill           = accentPrimary.withValues(alpha: 0.12);
```

### 2.2 Platform Badge Colors

```dart
Color platformMacOS       = Color(0xFFA2AAAD);  // Silver / macOS chrome
Color platformWindows     = Color(0xFF00A4EF);  // Windows blue
Color platformLinux       = Color(0xFFFF9F0A);  // Tux orange
Color platformAndroid     = Color(0xFF30D158);  // Android green
Color platformIOS         = Color(0xFF0A84FF);  // iOS blue
```

### 2.3 Color Usage Rules

1. **Never use raw `Colors.grey[500]`** — always use semantic text/surface colors
2. **Accent is sparing** — use for primary actions, active states, key highlights only
3. **Status colors are for indicators** — dots, badges, inline tags, never backgrounds
4. **Fill colors are 12% opacity** — use the `Fill` variants for container backgrounds
5. **Text contrast must meet WCAG AA** — textPrimary #FFF, textSecondary #E5E5EA, textTertiary #8E8E93 all pass

---

## 3. Typography System

### 3.1 Type Scale (SF Pro inspired)

```dart
TextStyle displayLarge   = TextStyle(  // Large screen titles — rarely used
  fontSize: 34, weight: FontWeight.w700, letterSpacing: 0.37, height: 1.15,
);
TextStyle displayMedium  = TextStyle(  // Page titles — "Devices", "Settings"
  fontSize: 24, weight: FontWeight.w700, letterSpacing: 0.0, height: 1.2,
);
TextStyle displaySmall   = TextStyle(  // Section titles — "Nearby Devices"
  fontSize: 20, weight: FontWeight.w600, letterSpacing: 0.0, height: 1.2,
);
TextStyle headlineLarge  = TextStyle(  // Card titles — Device name
  fontSize: 17, weight: FontWeight.w600, letterSpacing: -0.2, height: 1.25,
);
TextStyle headlineMedium = TextStyle(  // Subheadings / group headers
  fontSize: 15, weight: FontWeight.w600, letterSpacing: -0.2, height: 1.3,
);
TextStyle headlineSmall  = TextStyle(  // Small group labels
  fontSize: 14, weight: FontWeight.w600, letterSpacing: -0.1, height: 1.3,
);
TextStyle bodyLarge      = TextStyle(  // Primary body text
  fontSize: 14, weight: FontWeight.w400, letterSpacing: -0.1, height: 1.4,
);
TextStyle bodyMedium     = TextStyle(  // Secondary body / subtitles
  fontSize: 13, weight: FontWeight.w400, letterSpacing: -0.1, height: 1.35,
);
TextStyle bodySmall      = TextStyle(  // Captions / metadata
  fontSize: 12, weight: FontWeight.w400, letterSpacing: 0.0, height: 1.3,
);
TextStyle labelLarge     = TextStyle(  // Button labels / strong emphasis
  fontSize: 13, weight: FontWeight.w600, letterSpacing: 0.3, height: 1.25,
);
TextStyle labelMedium    = TextStyle(  // Badges / pills / small UI text
  fontSize: 11, weight: FontWeight.w500, letterSpacing: 0.3, height: 1.2,
);
TextStyle labelSmall     = TextStyle(  // Tiny captions / timestamps
  fontSize: 10, weight: FontWeight.w500, letterSpacing: 0.2, height: 1.2,
);
```

### 3.2 Font Family

- **Primary**: `'.SF Pro Display'` (macOS), `'Segoe UI Variable'` (Windows) via system font fallback
- **Monospace (codes/IDs)**: `'.SF Mono'` / `'Cascadia Code'` — only in developer-accessible areas

### 3.3 Typography Rules

1. **Never hardcode font sizes** — always use the type scale
2. **Line height** is always explicit — no auto
3. **Letter spacing** follows Apple's convention: positive for small, negative for large
4. **Color** is always from the text palette — `textPrimary`, `textSecondary`, `textTertiary`, `textQuaternary`
5. **Font weight**: 700 for display, 600 for headlines/labels, 500 for emphasis, 400 for body

---

## 4. Spacing & Layout

### 4.1 Base Unit

4px base grid. All spacing is a multiple of 4.

```dart
// Spacing scale (all values are multiples of 4)
const double spaceXXS = 2;    // 2px  — micro adjustments
const double spaceXS  = 4;    // 4px  — tight icon-text gaps
const double spaceSM  = 8;    // 8px  — tight spacing
const double spaceMD  = 12;   // 12px — standard small gap
const double spaceLG  = 16;   // 16px — standard padding
const double spaceXL  = 20;   // 20px — comfortable padding
const double space2XL = 24;   // 24px — section spacing
const double space3XL = 32;   // 32px — large section breaks
const double space4XL = 40;   // 40px — screen edge margins
const double space5XL = 48;   // 48px — major sections
const double space6XL = 64;   // 64px — page-level spacing
```

### 4.2 Layout Grid

```
Window minimum size: 900 × 600
Sidebar width:        220px (collapsible to 56px icon-only)
Detail panel width:   320px
Content padding:      32px from edges
Card grid gap:        16px
```

### 4.3 Content Widths

- **Device cards**: 280px wide (3 columns at 900px window)
- **Clipboard items**: Full width of content area
- **Settings**: Full width, max 640px for readability
- **Detail panel**: 320px fixed

---

## 5. Elevation & Shadows

### 5.1 Shadow Scale

```dart
List<BoxShadow> shadowNone    = [];  // Flat — default state
List<BoxShadow> shadowSM     = [     // Subtle — resting cards
  BoxShadow(color: black(0.10), blurRadius: 4,  offset: Offset(0, 1)),
  BoxShadow(color: black(0.06), blurRadius: 2,  offset: Offset(0, 0)),
];
List<BoxShadow> shadowMD     = [     // Medium — hovered cards
  BoxShadow(color: black(0.18), blurRadius: 12, offset: Offset(0, 4)),
  BoxShadow(color: black(0.08), blurRadius: 4,  offset: Offset(0, 1)),
];
List<BoxShadow> shadowLG     = [     // Large — modal / popover / flyout
  BoxShadow(color: black(0.24), blurRadius: 24, offset: Offset(0, 8)),
  BoxShadow(color: black(0.10), blurRadius: 8,  offset: Offset(0, 2)),
];
List<BoxShadow> shadowXL     = [     // Extra — full-screen overlay
  BoxShadow(color: black(0.32), blurRadius: 48, offset: Offset(0, 16)),
  BoxShadow(color: black(0.12), blurRadius: 16, offset: Offset(0, 4)),
];
```

### 5.2 Shadow Rules

1. **Shadows use two layers** — diffuse (soft, far) and sharp (tight, close) for natural depth
2. **Cards resting** use `shadowSM`
3. **Cards hovered** lift to `shadowMD` (paired with -2px Y transform)
4. **Modals/Dialogs** use `shadowLG`
5. **Overlays** use `shadowXL`
6. **Never use `elevation` from Material** — always explicit `BoxShadow`
7. **Animate shadow transitions** with `TweenAnimationBuilder` or `AnimatedContainer`

---

## 6. Radius & Shape Scale

```dart
double radiusNone      = 0;     // Rectangles
double radiusXS       = 4;     // Badges, small indicators
double radiusSM       = 6;     // Buttons, small containers
double radiusMD       = 8;     // Input fields, navigation items
double radiusLG       = 10;    // Cards
double radiusXL       = 12;    // Large containers, panels
double radius2XL      = 16;    // Detail panel icons, prominent items
double radius3XL      = 20;    // Modals, sheet corners
double radius4XL      = 28;    // Large OS badges
double radiusFull     = 999;   // Pills, circular elements
```

---

## 7. Glass & Blur Effects

### 7.1 When to Use Glass

- **Sidebar** — subtle glass background
- **Header bar** — glass overlay when content scrolls behind
- **Modals & sheets** — glass with backdrop blur
- **Detail panel** — glass separator from main content

### 7.2 Glass Specification

```dart
// Sidebar glass
BoxDecoration sidebarGlass = BoxDecoration(
  gradient: LinearGradient(
    begin: Alignment.topLeft, end: Alignment.bottomRight,
    colors: [
      Color(0xFF101012).withValues(alpha: 0.78),
      Color(0xFF0E0E10).withValues(alpha: 0.85),
    ],
  ),
  // Note: Apply ImageFiltered for actual blur
);

// Header glass — blurred backdrop when content scrolls
BoxDecoration headerGlass = BoxDecoration(
  color: Color(0xFF0A0A0B).withValues(alpha: 0.82),
  // backdropFilter: ImageFilter.blur(sigmaX: 20, sigmaY: 20),
);

// Modal backdrop
BoxDecoration modalBackdrop = BoxDecoration(
  color: Color(0xFF000000).withValues(alpha: 0.4),
  // backdropFilter: ImageFilter.blur(sigmaX: 4, sigmaY: 4),
);
```

**Implementation note**: Use `ClipRRect` + `BackdropFilter` for blur effects. On macOS, this is performant. On Windows, consider disabling blur or using a fallback solid color.

---

## 8. Iconography

### 8.1 Icon System

- **Primary tool icons**: SF Symbols-style — use `CupertinoIcons` or custom SVG icons
- **Status indicators**: 6px dots with pulse animation
- **Platform badges**: Mono-color, simplified versions of platform branding
- **Quick action icons**: Minimal, 16px, same stroke width

### 8.2 Icon Sizes

```dart
double iconTiny   = 12;  // Inline metadata
double iconSM     = 14;  // Button icons
double iconMD     = 16;  // Standard icon size
double iconLG     = 18;  // Navigation icons
double iconXL     = 20;  // Section icons
double icon2XL    = 24;  // Featured icons
double icon3XL    = 32;  // Empty state / hero
double icon4XL    = 48;  // Large device icons
```

### 8.3 Icon Guidelines

1. **All icons must be outlined** (filled only for active/selected states)
2. **Stroke width**: 1.5pt for 16px, 2pt for 20px+
3. **Consistent weight** across the entire app
4. **No filled icons** except for active nav items and toggle states
5. **Platform icons** are minimal — just the distinguishing silhouette

---

## 9. Motion System

### 9.1 Animation Curves

```dart
// Standard easing — for most UI animations
Curve easingStandard  = Curves.easeInOutCubic;  // (0.65, 0, 0.35, 1)

// Entrance — elements appearing on screen
Curve easingEnter     = Curves.easeOutCubic;     // (0.33, 1, 0.68, 1)

// Exit — elements leaving screen
Curve easingExit      = Curves.easeInCubic;      // (0.32, 0, 0.67, 0)

// Spring — physics-based interactions
// Use with SpringDescription:
//   SpringDescription(mass: 1, stiffness: 180, damping: 20)

// Emphasized — Apple's emphasized easing for large movements
Curve easingEmphasized = Cubic(0.2, 0.0, 0.0, 1.0);
```

### 9.2 Duration Scale

```dart
Duration durationInstant   = 0.ms;       // No animation (reduced motion)
Duration durationFast      = 100.ms;     // Micro-interactions
Duration durationNormal    = 200.ms;     // Most UI transitions
Duration durationSlow      = 350.ms;     // Large movements, panel slides
Duration durationEmphasized = 500.ms;    // Emphasized transitions
```

### 9.3 Animation Types

#### Hover
- **Trigger**: Mouse enters element
- **Effect**: Lift (-2px Y), shadow SM → MD, border accent glow
- **Duration**: 150ms
- **Curve**: easingStandard
- **Targets**: Device cards, action buttons, settings tiles, nav items

#### Selection / Tap
- **Trigger**: Mouse click / tap
- **Effect**: Brief scale dip (1.0 → 0.97 → 1.0), change to selected state
- **Duration**: 200ms
- **Curve**: spring(mass:1, stiffness: 300, damping: 25)
- **Targets**: Navigation items, device selection, toggles

#### Entrance (content appearing)
- **Trigger**: Screen loads, section appears
- **Effect**: Fade in + slide up (0, 8px → identity)
- **Duration**: 350ms, staggered by 50ms per child
- **Curve**: easingEnter
- **Targets**: New screens, device cards appearing, list items

#### Exit (content disappearing)
- **Trigger**: Screen leaves, item removed
- **Effect**: Fade out + slide down (identity → 0, -8px)
- **Duration**: 200ms
- **Curve**: easingExit
- **Targets**: Dismissed panels, removed items

#### Panel Slide (side panel)
- **Trigger**: Device selected
- **Effect**: Slide in from right (translateX: panelWidth → 0)
- **Duration**: 300ms
- **Curve**: easingEmphasized
- **Targets**: Detail panel, settings sub-pages

#### Pulse (connection / discovery)
- **Trigger**: Device discovered, connection established
- **Effect**: Scale pulse (1.0 → 1.05 → 1.0) with glow
- **Duration**: 600ms loop
- **Curve**: easingStandard
- **Targets**: Discovery indicator, online dot, pairing animation

#### Status Dot (online indicator)
- **Trigger**: Connection state change
- **Effect**: Scale up (0 → 1) + color transition
- **Duration**: 300ms
- **Curve**: spring(mass:1, stiffness: 200, damping: 20)
- **Targets**: All status indicators

#### Success (pairing / action complete)
- **Trigger**: Pairing successful, action completed
- **Effect**: Checkmark draw animation + brief glow + confetti
- **Duration**: 600ms
- **Curve**: easingEmphasized
- **Targets**: Pairing success, rename success, action confirmation

### 9.4 Reduced Motion

When `MediaQuery.reducedMotion` is enabled:
1. Remove all entrance/exit slide animations (keep fade)
2. Remove hover lift transforms
3. Remove pulse/glow animations
4. Keep color transitions (they don't cause motion sickness)
5. Duration: 0ms for all transforms, 100ms for fades
6. Status dot animations: instant opacity change only

---

## 10. Component Architecture

### 10.1 Screen Hierarchy

```
UniKeyApp
├── HomeScreen
│   ├── Sidebar
│   │   ├── AppLogo (UniKey branding)
│   │   ├── NavItem × N (Devices, Pair, Settings)
│   │   └── SidebarFooter (daemon status indicator)
│   ├── MainContent (routed based on _selectedIndex)
│   │   ├── DevicesView
│   │   │   ├── DeviceListHeader (title + actions)
│   │   │   ├── DiscoveryRadar (animated discovery indicator)
│   │   │   ├── DeviceCardGrid (virtualized)
│   │   │   │   └── DeviceCard × N
│   │   │   ├── TrustedDevicesSection
│   │   │   │   └── TrustedDeviceRow × N
│   │   │   └── EmptyState (illustration + text)
│   │   ├── PairingView
│   │   │   ├── PairingTabs (Show Code / Enter Code)
│   │   │   ├── ShowCodePanel
│   │   │   │   ├── AnimatedQRCode
│   │   │   │   ├── PairCodeDisplay
│   │   │   │   ├── CountdownRing
│   │   │   │   └── CopyButton
│   │   │   ├── EnterCodePanel
│   │   │   │   ├── CodeInput (6 separate fields)
│   │   │   │   ├── PeerSelector
│   │   │   │   └── PairButton
│   │   │   └── PairingSuccessAnimation
│   │   ├── SettingsView
│   │   │   ├── SettingsSearch
│   │   │   ├── SettingsCategory × N
│   │   │   │   └── SettingsRow × N
│   │   │   └── SettingsSubPage (pushed navigation)
│   │   ├── ClipboardView
│   │   │   ├── ClipboardSearch
│   │   │   ├── ClipboardFilter
│   │   │   ├── ClipboardGrid (virtualized)
│   │   │   │   └── ClipboardItem × N (text/image/file)
│   │   │   └── ClipboardEmptyState
│   │   └── InputForwardingView
│   │       ├── ForwardToggle
│   │       ├── TargetDeviceSelector
│   │       └── ForwardStatusIndicator
│   └── DetailPanel
│       ├── PanelHeader (device name + close)
│       ├── DeviceAvatar (large icon + status)
│       ├── DeviceInfoGrid (battery, latency, quality, OS)
│       ├── QuickActions (send file, clipboard, connect)
│       └── DeviceManagement (rename, forget, trust)
```

### 10.2 Shared / Reusable Components

| Component | Used In | Props |
|-----------|---------|-------|
| `UniKeyIcon` | Everywhere | `icon`, `size`, `color`, `filled` |
| `StatusDot` | Sidebar, cards, detail panel | `status`, `size`, `animated` |
| `PlatformBadge` | Cards, detail panel, clipboard | `platform`, `size` |
| `SectionHeader` | All views | `title`, `count`, `action` |
| `EmptyState` | Devices, clipboard, search | `illustration`, `title`, `subtitle`, `action` |
| `UniKeyCard` | Devices, clipboard | `child`, `padding`, `onTap`, `onHover` |
| `UniKeyButton` | Everywhere | `variant`, `label`, `icon`, `onPressed`, `loading` |
| `UniKeyDialog` | Rename, confirm | `title`, `content`, `actions` |
| `HoverRegion` | Any hoverable area | `child`, `onHover`, `transform` |
| `AnimatedEntrance` | List items | `child`, `delay`, `animation` |
| `LoadingShimmer` | Loading states | `width`, `height`, `borderRadius` |

### 10.3 State Management Architecture

```
DeviceProvider (ChangeNotifier)
├── DaemonConnectionState (connecting, connected, disconnected, error)
├── Device myDevice
├── List<Device> nearbyDevices
├── List<Device> trustedDevices
├── ClipboardHistory clipboardHistory
├── SettingsState settings
├── PairingSession? activeSession
└── Methods: refresh, pair, rename, remove, forwardInput, ...

ClipboardProvider (ChangeNotifier)
├── List<ClipboardEntry> entries
├── bool isLoading
├── String searchQuery
├── Set<String> pinnedIds
└── Methods: sync, pin, unpin, delete, search, clear

SettingsProvider (ChangeNotifier)
├── bool clipboardSyncEnabled
├── bool fileSharingEnabled
├── bool launchAtStartup
├── bool keyboardForwardingEnabled
├── ThemeMode themeMode
└── Methods: toggle, update

PairingProvider (ChangeNotifier)
├── String? generatedCode
├── int expiresIn
├── bool isGenerating
├── bool isPairing
├── PairingResult? result
└── Methods: generateCode, initiatePairing, reset
```

---

## 11. Screen Specifications

### 11.1 Sidebar

```
┌─────────────────────────────────────┐
│ [K] UniKey                          │  ← AppLogo section
├─────────────────────────────────────┤
│                                     │
│  ◇  Devices                         │  ← NavItem (selected: accent)
│  +  Pair Device                     │  ← NavItem (default: textTertiary)
│  ⚙  Settings                        │  ← NavItem
│                                     │
│                                     │  ← Spacer
│                                     │
│ ●  Daemon Connected                 │  ← SidebarFooter
└─────────────────────────────────────┘
    220px
```

**Interaction**:
- Selected item: accent text, subtle accent fill background, active indicator dot
- Hover: subtle white fill at 4% opacity
- Click: spring scale tap response
- Footer shows daemon connection status with live animated dot

**Animation**:
- Nav items: background color 150ms, text color 150ms
- Active indicator: crossfade between items
- Status dot: pulse every 2 seconds when connected

**States**:
- Active nav: accentPrimary + 12% accentFill
- Inactive nav: textTertiary
- Hover inactive: 4% white fill
- Daemon connected: green, pulse
- Daemon disconnected: red
- Daemon connecting: orange, slow pulse

### 11.2 Device List (Main Content)

```
┌────────────────────────────────────────────────────────┐
│  Devices                              [Refresh] [Sort]  │  ← Header
├────────────────────────────────────────────────────────┤
│                                                        │
│  ╭────────────────────────────────────────────╮        │
│  │  ● ● ●  Scanning...                       │        │  ← DiscoveryRadar (when searching)
│  │                                            │        │
│  ╰────────────────────────────────────────────╯        │
│                                                        │
│  Nearby Devices                    3                    │  ← SectionHeader
│                                                        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐               │
│  │ 💻       │ │ 🖥       │ │ 📱       │               │  ← DeviceCard × N
│  │ MacBook  │ │ Windows  │ │ iPhone   │               │     (virtualized grid)
│  │ Pro      │ │ Desktop  │ │ 16       │               │
│  │ ● Online │ │ ● Online │ │ ○ Offline│               │
│  │ [Connect]│ │ [Connect]│ │ [Pair]   │               │
│  └──────────┘ └──────────┘ └──────────┘               │
│                                                        │
│  Trusted Devices                    2                   │  ← SectionHeader
│                                                        │
│  ┌─────────────────────────────────────────────────┐   │
│  │ 💻 MacBook Pro     ● Online               ›    │   │  ← TrustedDeviceRow
│  ├─────────────────────────────────────────────────┤   │
│  │ 🖥 Windows Desktop ● Online               ›    │   │
│  └─────────────────────────────────────────────────┘   │
│                                                        │
└────────────────────────────────────────────────────────┘
```

**Interaction**:
- Card tap: selects device, opens detail panel
- Card hover: lifts, shows quick actions overlay
- Trusted row tap: opens detail panel
- "Connect" button: establishes active control session
- "Pair" button: switches to PairingView with peer pre-selected

**Animation**:
- Cards animate in staggered (50ms delay) with fade+slide
- New device discovered: pulse glow + slide into grid
- Device goes offline: status dot color transition 300ms
- Grid items animate to new positions on reorder

**Empty State**:
```
┌──────────────────────────────────────────────┐
│                                              │
│          ┌──────────────────┐                │
│          │   [Illustration] │                │
│          │  Radar/Network   │                │
│          │  Animation       │                │
│          └──────────────────┘                │
│                                              │
│    No devices found nearby                   │
│    Make sure UniKey is running on            │
│    your other devices and they're            │
│    connected to the same network.            │
│                                              │
│    [Refresh]  [Learn More]                   │
│                                              │
└──────────────────────────────────────────────┘
```

### 11.3 Device Card

```
┌──────────────────────┐
│  ┌────┐    ● Online  │  ← Status dot + label
│  │ 💻 │              │  ← Platform icon in glass container
│  └────┘              │
│                      │
│  MacBook Pro          │  ← device name (headlineLarge)
│  macOS 14.3           │  ← OS version (bodySmall)
│  192.168.1.42         │  ← Network (bodySmall, subtle)
│                      │
│  ▮▮▮▮▮▮▮▮▮▮ 85%      │  ← Battery indicator (if available)
│  ●━━━━━━━ 2ms latency │  ← Connection quality
│                      │
│  [Connect]  [⋮]      │  ← Primary action + menu
└──────────────────────┘
  280px
```

**Hover state**:
- Card lifts (-2px Y) with shadow SM → MD (200ms)
- Border accent glows subtle green
- Quick action overlay appears (Connect button, menu icon)
- Battery bar animates slightly wider

**Selected state**:
- Border turns accent color
- Subtle accent fill background
- Check indicator in top-right corner

### 11.4 Detail Panel

```
┌─────────────────────────────────┐
│  Device Info              [X]  │  ← PanelHeader
├─────────────────────────────────┤
│                                 │
│        ┌────────────┐          │
│        │    💻      │          │  ← DeviceAvatar (64px)
│        └────────────┘          │
│                                 │
│       MacBook Pro               │  ← device name
│          ● Online               │  ← StatusBadge
│                                 │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │
│                                 │
│  Platform     macOS 14.3        │  ← InfoGrid
│  Battery      85% — Charging    │
│  Latency      2ms               │
│  Connection   Excellent         │
│  Network      192.168.1.42      │
│  Trusted      Yes               │
│                                 │
│  ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─  │
│                                 │
│  Quick Actions                  │
│                                 │
│  [Send File] [Push Clipboard]   │
│  [Control Device]               │
│                                 │
│  Management                     │
│                                 │
│  ✏ Rename                       │
│  🔗 Unpair Device               │
│                                 │
└─────────────────────────────────┘
  320px
```

**Interaction**:
- Slides in from right with emphasized easing (300ms)
- Content fades in with 50ms stagger
- Tap "Rename": inline edit or bottom sheet
- Tap "Unpair": confirmation with destructive styling
- Quick actions trigger with brief success animation

### 11.5 Pairing View

#### Show Code Tab

```
┌──────────────────────────────────────┐
│  Show pairing code on this device    │
│                                      │
│  ┌──────────────────────────────┐   │
│  │                              │   │
│  │      ┌──────────────┐       │   │  ← AnimatedQR
│  │      │ ██▓▒░  ░▒▓██ │       │   │     (real QR code)
│  │      │ ░▒▓██  ██▓▒░ │       │   │
│  │      └──────────────┘       │   │
│  │                              │   │
│  │        4 8 2 1 9 3          │   │  ← PairCodeDisplay
│  │                              │   │     (large, spaced, monospace)
│  │    ┌──────────────────┐      │   │
│  │    │    Code expires   │      │   │  ← CountdownRing
│  │    │    in 45s  ◯      │      │   │     (animated ring)
│  │    └──────────────────┘      │   │
│  │                              │   │
│  │         [Copy Code]          │   │  ← CopyButton
│  └──────────────────────────────┘   │
│                                      │
│  [Generate New Code]                 │
└──────────────────────────────────────┘
```

**CountdownRing animation**:
- Circular progress ring that depletes over 60s
- Color: accentPrimary → orangeWarning → errorRed below 10s
- Smooth linear animated progress
- When expired: ring empties, code fades out, "Expired" label appears

#### Enter Code Tab

```
┌──────────────────────────────────────┐
│  Enter code from your other device   │
│                                      │
│  ┌──────────────────────────────┐   │
│  │   [_][_][_][_][_][_]        │   │  ← CodeInput (6 fields)
│  │    │  │  │  │  │  │         │   │     auto-advance on type
│  └──────────────────────────────┘   │
│                                      │
│  Target Device                       │
│                                      │
│  ┌──────────────────────────────┐   │
│  │ 💻 MacBook Pro           ●  │   │  ← PeerSelector
│  ├──────────────────────────────┤   │
│  │ 🖥 Windows Desktop       ●  │   │
│  └──────────────────────────────┘   │
│                                      │
│  [Pair Devices]                      │  ← PairButton
│                                      │
│  ✓ Successfully paired!              │  ← SuccessAnimation
│     with MacBook Pro                  │     (checkmark + glow)
│                                      │
└──────────────────────────────────────┘
```

**Success Animation**:
1. Button transforms to loading spinner
2. On success: brief green flash
3. Checkmark draws in (stroke animation, 400ms)
4. Device card scales up + glow
5. Haptic-like bounce on completion
6. 2s delay, then auto-navigate to Devices

### 11.6 Clipboard View

```
┌──────────────────────────────────────────────┐
│  Clipboard                         [Clear]   │
├──────────────────────────────────────────────┤
│  🔍 Search clipboard...                      │  ← SearchBar
├──────────────────────────────────────────────┤
│                                              │
│  Today                                       │  ← Timeline Group
│                                              │
│  ┌────────────────────────────────────┐     │
│  │  const ApiClient._baseUrl =        │     │  ← ClipboardItem
│  │  'http://127.0.0.1...              │     │     (text with preview)
│  │                       13:42 · 💻   │     │
│  │                        📌 ↗ 📋    │     │  ← Actions: pin, open, copy
│  └────────────────────────────────────┘     │
│                                              │
│  ┌────────────────────────────────────┐     │
│  │  [Image Thumbnail]   Flutter       │     │  ← ClipboardItem
│  │  Rust Bridge integration guide     │     │     (image with text)
│  │                       13:40 · 🖥   │     │
│  └────────────────────────────────────┘     │
│                                              │
│  Earlier Today                               │
│                                              │
│  ┌────────────────────────────────────┐     │
│  │  https://github.com/...github      │     │  ← ClipboardItem
│  │                       13:35 · 💻   │     │     (link preview)
│  └────────────────────────────────────┘     │
│                                              │
└──────────────────────────────────────────────┘
```

**Features**:
- Search filters in real-time
- Pin items (sticky to top)
- Copy on tap
- Preview images inline
- Preview links with domain favicon
- Timeline grouping (Today, Yesterday, This Week, Older)
- Source device icon per item
- Drag-and-drop from clipboard onto desktop

### 11.7 Settings View

```
┌──────────────────────────────────────────────┐
│  Settings                    🔍 Search...    │
├──────────────────────────────────────────────┤
│                                              │
│  GENERAL                                     │  ← Category
│                                              │
│  ┌────────────────────────────────────┐     │
│  │ 🚀  Launch at startup          ON │     │  ← SettingsRow
│  │     Automatically start on login   │     │     (with toggle)
│  ├────────────────────────────────────┤     │
│  │ 📋  Share clipboard            ON │     │
│  │     Sync across your devices       │     │
│  ├────────────────────────────────────┤     │
│  │ 📁  File Sharing               OFF│     │
│  │     Transfer between devices       │     │
│  └────────────────────────────────────┘     │
│                                              │
│  APPEARANCE                                  │  ← Category
│                                              │
│  ┌────────────────────────────────────┐     │
│  │ 🎨  Appearance              Dark   │     │  ← SettingsRow
│  │     Theme selection                │     │     (with value → subpage)
│  ├────────────────────────────────────┤     │
│  │ 🔲  Icon Style          Monochrome│     │
│  └────────────────────────────────────┘     │
│                                              │
│  SECURITY                                    │
│                                              │
│  ┌────────────────────────────────────┐     │
│  │ 🔐  Encryption            Ed25519  │     │  → subpage
│  │     Key pair for device auth       │     │
│  ├────────────────────────────────────┤     │
│  │ 👤  Trusted Devices           5    │     │  → subpage
│  │     Manage paired devices          │     │
│  └────────────────────────────────────┘     │
│                                              │
│  NETWORK                                     │
│                                              │
│  ┌────────────────────────────────────┐     │
│  │ 🌐  Port                   45895   │     │
│  │     TCP communication port         │     │
│  ├────────────────────────────────────┤     │
│  │ 📶  Discovery Interface Auto      │     │
│  └────────────────────────────────────┘     │
│                                              │
└──────────────────────────────────────────────┘
```

**Interaction**:
- Rows with toggle: instant toggle animation
- Rows with value (→): tap pushes to sub-page with slide-right transition
- Search filters visible rows in real-time
- Categories collapse with tap on header
- Back navigation from sub-pages with slide-left gesture

### 11.8 Status Dot

```
● ● ●
Green  Orange  Red/Grey

Size: 8px (standard), 6px (inline)
Pulse: scale 1.0→1.3→1.0 over 2s (when active)
```

**States**:
| State | Color | Animation |
|-------|-------|-----------|
| Online / Connected | green | Subtle pulse (2s loop) |
| Busy / In Use | orange | Medium pulse (1.5s loop) |
| Offline | grey | None |
| Error | red | Fast pulse (0.8s loop) |
| Connecting | orange | Slow pulse (3s loop) |

---

## 12. State System

### 12.1 Every Component Has 4 States

Each interactive component must handle:

1. **Default** — Resting appearance
2. **Hover** — Mouse is over (desktop only)
3. **Active / Selected** — Currently active
4. **Disabled** — Cannot interact

### 12.2 Every View Has 5 States

Each data-driven view must handle:

1. **Loading** — Shimmer / skeleton / spinner
2. **Loaded (Data)** — Content with data
3. **Loaded (Empty)** — Empty state illustration
4. **Error** — Error illustration + retry
5. **Offline** — Daemon disconnected state

### 12.3 Loading States

```dart
// ShimmerPlaceholder — used for skeleton loading
Widget shimmerPlaceholder({
  double width = double.infinity,
  required double height,
  double borderRadius = 8,
}) {
  return Container(
    width: width,
    height: height,
    decoration: BoxDecoration(
      color: surfaceTertiary,
      borderRadius: BorderRadius.circular(borderRadius),
    ),
    // Overlay with animated shimmer gradient
  );
}
```

### 12.4 Error States

Every error state includes:
1. Illustration or icon (48px)
2. Error title (headlineLarge)
3. Error description (bodyMedium, textTertiary)
4. Action button (Retry or Dismiss)

---

## 13. Accessibility

### 13.1 Keyboard Navigation

| Key | Action |
|-----|--------|
| `Tab` | Move forward through focusable elements |
| `Shift+Tab` | Move backward |
| `↑/↓` | Navigate lists (device list, settings) |
| `←/→` | Navigate sidebar / tabs |
| `Enter/Space` | Activate selected item |
| `Escape` | Close panel / dismiss modal / go back |
| `Cmd+,` | Open settings |
| `Cmd+F` | Focus search (when available) |
| `Cmd+R` | Refresh devices |

**Focus indicator**: 2px solid accentPrimary ring with 4px gap from element edge.

### 13.2 Screen Reader Support

```dart
// Every interactive widget must have:
Semantics(
  label: 'MacBook Pro, online, 85% battery',
  hint: 'Tap to view device details',
  onTap: () => selectDevice(device),
  child: DeviceCard(device: device),
);
```

### 13.3 Reduced Motion

```dart
// Use this pattern for all animations:
final reduceMotion = MediaQuery.of(context).reduceMotion;

AnimatedContainer(
  duration: reduceMotion ? Duration.zero : Duration(milliseconds: 200),
  transform: reduceMotion ? Matrix4.identity() : _hoverTransform,
  // ...
);
```

### 13.4 Visual Accessibility

1. **Minimum touch target**: 32×32px for all interactive elements
2. **Text contrast**: All text meets WCAG AA (4.5:1 for normal, 3:1 for large)
3. **Status not conveyed by color alone**: Every status has text label + icon + color
4. **Focus order**: Logical left-to-right, top-to-bottom
5. **Font scaling**: Respects system font size settings

---

## 14. File Structure

```
lib/
├── main.dart
├── app.dart                           # UniKeyApp widget
│
├── theme/
│   ├── app_theme.dart                 # ThemeData construction
│   ├── colors.dart                    # All color constants
│   ├── typography.dart                # All text styles
│   ├── spacing.dart                   # Spacing constants
│   ├── shadows.dart                   # Shadow definitions
│   └── radius.dart                    # Radius scale
│
├── models/
│   ├── device.dart
│   ├── platform_type.dart
│   ├── connection_status.dart
│   └── clipboard_entry.dart
│
├── providers/
│   ├── device_provider.dart
│   ├── clipboard_provider.dart
│   ├── settings_provider.dart
│   └── pairing_provider.dart
│
├── services/
│   ├── api_client.dart
│   └── clipboard_service.dart
│
├── screens/
│   ├── home_screen.dart               # Shell with sidebar + content + panel
│   ├── devices_view.dart              # Devices main content
│   ├── pairing_view.dart              # Pairing (show/enter code)
│   ├── settings_view.dart             # Settings main page
│   ├── settings_subpage.dart          # Generic settings subpage
│   └── clipboard_view.dart            # Clipboard history
│
├── widgets/
│   ├── layout/
│   │   ├── sidebar.dart
│   │   ├── header.dart
│   │   ├── detail_panel.dart
│   │   └── app_shell.dart             # Sidebar + content + detail layout
│   │
│   ├── device/
│   │   ├── device_card.dart
│   │   ├── device_card_grid.dart      # Virtualized grid
│   │   ├── trusted_device_row.dart
│   │   ├── device_avatar.dart
│   │   └── platform_badge.dart
│   │
│   ├── pairing/
│   │   ├── animated_qr_code.dart
│   │   ├── pair_code_display.dart
│   │   ├── countdown_ring.dart
│   │   ├── code_input.dart            # 6-field input
│   │   ├── peer_selector.dart
│   │   └── pairing_success.dart       # Success animation
│   │
│   ├── clipboard/
│   │   ├── clipboard_item.dart        # Text/image/file variants
│   │   └── clipboard_empty_state.dart
│   │
│   ├── settings/
│   │   ├── settings_category.dart
│   │   ├── settings_row.dart
│   │   └── settings_search.dart
│   │
│   ├── indicators/
│   │   ├── status_dot.dart
│   │   ├── status_badge.dart
│   │   ├── battery_indicator.dart
│   │   ├── connection_quality.dart
│   │   ├── discovery_radar.dart
│   │   └── latency_indicator.dart
│   │
│   ├── common/
│   │   ├── unikey_card.dart
│   │   ├── unikey_button.dart
│   │   ├── unikey_dialog.dart
│   │   ├── unikey_icon.dart
│   │   ├── section_header.dart
│   │   ├── animated_entrance.dart
│   │   ├── empty_state.dart
│   │   ├── error_state.dart
│   │   ├── loading_shimmer.dart
│   │   └── hover_region.dart
│   │
│   └── input_forwarding/
│       ├── input_forwarding_view.dart
│       ├── target_selector.dart
│       └── forward_status.dart
│
└── utils/
    ├── animation_utils.dart           # Shared curves, durations
    ├── format_utils.dart              # Timestamps, byte formatting
    └── platform_utils.dart            # Platform detection helpers
```

---

## Version Plan (Phase 3 Order)

### Screen 1: Foundation + Theme + Common Widgets
- `colors.dart`, `typography.dart`, `spacing.dart`, `shadows.dart`, `radius.dart`
- `app_theme.dart` — rebuild using new system
- All `common/` widgets: `UniKeyCard`, `UniKeyButton`, `UniKeyDialog`, etc.
- `indicators/`: `StatusDot`, `StatusBadge`, `BatteryIndicator`

### Screen 2: Sidebar + App Shell
- `sidebar.dart` — complete redesign
- `app_shell.dart` — layout with sidebar, content, detail panel
- `header.dart` — redesign

### Screen 3: Device Cards + Device List
- `device_card.dart` — complete redesign
- `device_card_grid.dart` — virtualized
- `trusted_device_row.dart`
- `platform_badge.dart`, `device_avatar.dart`
- `devices_view.dart` — main devices screen
- `discovery_radar.dart`, empty states

### Screen 4: Detail Panel
- `detail_panel.dart` — complete redesign
- `connection_quality.dart`, `latency_indicator.dart`

### Screen 5: Pairing
- All `pairing/` widgets
- `pairing_view.dart` — complete overhaul
- QR code generation, countdown ring, success animation

### Screen 6: Settings
- `settings_view.dart` — extracted from home_screen
- `settings_category.dart`, `settings_row.dart`
- `settings_provider.dart`

### Screen 7: Clipboard
- `clipboard_view.dart` — full redesign
- `clipboard_item.dart` with variants
- `clipboard_provider.dart`

### Screen 8: Input Forwarding
- `input_forwarding_view.dart` — consumer UX redesign
- `target_selector.dart`, `forward_status.dart`

> End of Phase 2 Redesign Document
