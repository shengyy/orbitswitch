# Design

This page owns the app and website's visual presentation and interaction, icons and screenshots.
Product behavior lives in [PRODUCT.md](../PRODUCT.md); build, signing and website publishing live in
[release.md](release.md).

OrbitSwitch should feel like part of the operating system: one glance to see every account's quota, one
confirmed click to switch.

## Surfaces

- **macOS**: a popover under the menu bar icon, Liquid Glass background, no Dock icon. It resizes to its
  content (up to 680 pt) and closes when focus leaves, like a menu. The WebView lives inside the native
  Liquid Glass container through `window-vibrancy`’s `content_view` API; that container clips all page
  fills and confirmation backdrops to the same 18 pt corners. The extra system window shadow is
  disabled. macOS 26 or later.
- **Windows**: a compact window opened from the tray icon. Closing it keeps OrbitSwitch in the tray.
- Both run the same React UI; `platform-macos` / `platform-windows` on the root only change surface
  tokens and sizing.
- Left click toggles the surface. Right click opens a native menu (open, quit).
- Open at Login shows OrbitSwitch by name and icon in macOS System Settings; on Windows it starts hidden
  in the tray. Registration is described in [architecture.md](architecture.md).

## Interaction

What happens and when is defined in [PRODUCT.md](../PRODUCT.md); this is how it is presented.

- The Gemini / ChatGPT / Claude tab bar selects a view only. Arrow keys, Home and End move between tabs. Each provider has its own current account and installed/running state.
- Confirmations are sheets that slide up inside the surface, never separate windows. Enter confirms,
  Escape cancels, and a confirmation names what it will close.
- Long operations show their real progress (switch steps come from the backend) and cannot be dismissed
  half-way. Success lingers 1.6 s; errors stay until acknowledged.
- While an account signs in, the surface shows a cancellable waiting sheet and comes back when Google
  redirects.
- Updates appear in a compact banner under the header. Clicking opens the existing sheet style with
  current and new versions, scrollable release notes, Later and Update & Restart. Downloads show real
  progress, including an indeterminate bar when the server gives no size; verification and installation
  have separate steps. The app menu also offers Check for Updates.
- Account actions (sign in again, copy email, remove) live in a native context menu: right-click a card
  or use its `⋯` button. App actions live in the header's `⋯` menu.

## Account card

- Avatar (Google photo, else initials), name, plan badge (provider-reported `PRO` / `ULTRA` / `PLUS` / `MAX` / team tiers), email.
- Tags follow the name and plan badge: `Current` for the account in use, `Most left` for the account
  worth switching to.
- One row per quota group (Gemini, Claude) with the 5-hour and weekly windows side by side, each a meter
  and the share remaining. Colour changes only when it matters: amber below 35 %, red below 10 %.
- When a window is low, a footnote says when it recovers; hovering any meter shows its reset time.
- Clicking a switchable card opens the switch confirmation. Hover and keyboard focus highlight its
  fill and border; the account identity is the keyboard activation target. More sits at the far right
  of the name and tags row and appears on hover or focus. Long names and emails truncate within their
  own column.

## Visual tokens

Defined once in `src/styles/tokens.css`: text and fill opacities, accent, meter colours, card and sheet
surfaces, light and dark. Components use tokens only. Type is the system font at 13 px (11–11.5 px for
secondary text) with tabular numerals for percentages.

## Icons

OrbitSwitch's rotary selector has three segmented arcs for the three platforms and a central pointer
for switching accounts. It has no planet, satellite or vendor-logo dependency. The colored mark is
owned by `assets/brand/app-icon.svg`; app UI and website reuse it. The matching transparent monochrome
mark is `assets/brand/tray-template.svg`, with solid paths and clear gaps for native tinting at 16/18px.
The charcoal tile uses quiet gray-green inactive arcs, a mineral-green active arc (`#82AA98`) and a
warm-white pointer (`#F3F0E8`). The website and app use the same mineral palette, with deeper green
for text and buttons on light backgrounds. No colored glow or complementary blue/orange gradient.
Quota warning and error colors remain semantic, independent of the brand palette.
Both were designed by Argon High and checked at 16, 18, 32, 128 and 1024px.

Regenerate the bundled icons after editing the SVGs:

```bash
rsvg-convert -w 1024 -h 1024 assets/brand/app-icon.svg -o /tmp/orbitswitch-icon.png
bun tauri icon /tmp/orbitswitch-icon.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios src-tauri/icons/Square*Logo.png src-tauri/icons/StoreLogo.png src-tauri/icons/64x64.png
rsvg-convert -h 36 assets/brand/tray-template.svg -o src-tauri/icons/tray-template.png
```

## Screenshots

README and website screenshots are rendered from the real UI with fictional accounts, so they never
show anyone's email, and always match the current design:

```bash
bun scripts/screenshot/render.ts   # needs Google Chrome and ImageMagick
```

It builds the frontend, replaces Tauri's IPC with `scripts/screenshot/mock-tauri.js` (demo data and
scenes), stages the popover under a menu bar with `frame.css`, renders at 3x and writes
`assets/screenshots/{hero,flow,codex,claude}-{light,dark}@3x.png` plus 2x copies without the suffix. The README uses
the 2x images; the website has a separate interactive fictional demo. Never commit screenshots of real accounts.

## Website

A restrained instrument-like layout combines generous whitespace, thin rules, compact technical
labels and an interactive menu-bar demo. `site/style.css` owns page layout and responsive rules;
`site/demo.css` owns the demo surface. `site/demo.js` owns fictional accounts, localized copy and
keyboard interaction. All demo accounts use `example.com`; they are visibly labeled as demo data.
The English/Chinese switch, light/dark appearance, focus states and reduced-motion preference apply
across the page. Technical implementation details are kept out of the primary product flow.

The download resolver distinguishes the legacy 0.1.x release from a multi-platform release. Publishing
and local preview are described in [release.md](release.md#website).
