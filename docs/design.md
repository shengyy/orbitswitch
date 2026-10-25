# Design

Visual presentation, interaction, icons and screenshots for the app and website. Product rules live in
[PRODUCT.md](../PRODUCT.md); build, signing and publishing live in [release.md](release.md).

## Surfaces

- **macOS (26+)**: menu bar popover with no Dock icon, sized to content (up to 720 pt) and dismissed
  on focus loss. The WebView is mounted inside the native Liquid Glass container via
  `window-vibrancy`'s `content_view` API with 18 pt corner clipping and no extra system window shadow.
- **Windows**: compact tray window that hides to the tray on close.
- Both platforms share the React UI with `platform-macos` / `platform-windows` root classes. Left
  click toggles the window; right click opens the native tray menu.

## Interaction

- The Gemini / ChatGPT / Claude tab bar switches views; Arrow keys, Home and End navigate between
  tabs.
- Confirmations, progress and update details slide up as in-surface sheets (`Enter` confirms,
  `Escape` cancels). Confirmations list the affected processes.
- Long-running operations show backend step progress; success closes after 1.6 s and errors remain
  until dismissed.
- Browser sign-in shows a cancellable waiting sheet until authentication completes.
- Account actions (`Sign In Again…`, `Copy Email`, `Remove Account…`) open in a native context menu
  on right click or `⋯`. App actions live in the header's `⋯` menu.

## Account card

- Avatar (profile photo or initial), display name, plan badge (`PRO`, `ULTRA`, `PLUS`, `MAX`, team
  tiers) and email.
- `Current` marks the active account; `Most left` marks the recommended switch target.
- Quota rows render each reported group and window meter with remaining percentage. Meters turn amber
  below 35 % and red below 10 %, with a low-quota recovery footnote and reset time on hover.
- Hover or keyboard focus highlights switchable cards; clicking or activating the account identity
  opens the switch confirmation.

## Visual tokens and icons

- `src/styles/tokens.css` defines all light/dark surface, text, mineral-green accent (`#476f5b` /
  `#82aa98`) and semantic status tokens. Typography uses the system font at 13 px (11–11.5 px
  secondary) with tabular numerals.
- `assets/brand/app-icon.svg` owns the three-segment rotary selector app icon;
  `assets/brand/tray-template.svg` owns the monochrome tray template mark.

Regenerate bundled icons after editing the SVGs:

```bash
rsvg-convert -w 1024 -h 1024 assets/brand/app-icon.svg -o /tmp/orbitswitch-icon.png
bun tauri icon /tmp/orbitswitch-icon.png -o src-tauri/icons
rm -rf src-tauri/icons/android src-tauri/icons/ios src-tauri/icons/Square*Logo.png src-tauri/icons/StoreLogo.png src-tauri/icons/64x64.png
rsvg-convert -h 36 assets/brand/tray-template.svg -o src-tauri/icons/tray-template.png
```

## Screenshots

Render README and website screenshots from the real UI with fictional accounts:

```bash
bun scripts/screenshot/render.ts   # requires Google Chrome and ImageMagick
```

Writes `assets/screenshots/{hero,flow,codex,claude}-{light,dark}@3x.png` and 2x copies without the
suffix using `scripts/screenshot/mock-tauri.js` and `frame.css`.

## Website

`site/index.html`, `site/style.css`, `site/demo.css`, `site/demo.js` and `site/download.js` implement
the bilingual landing page, interactive fictional `@example.com` demo and GitHub Releases installer
resolver. Local preview and deployment live in [release.md](release.md#website).
