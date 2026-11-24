# Code map

The UI is organized by responsibility. `App.tsx` still owns wizard state and
native subscriptions; reusable presentation and pure rules live outside it.
This keeps behavior stable while making common changes smaller and easier to test.

## Where to change each feature

| Feature | File or directory |
| --- | --- |
| Platforms, formats, qualities, cookie options and step labels | `src/config/downloadOptions.ts` |
| Shared spring transition | `src/config/motion.ts` |
| Selection cards | `src/components/ChoiceCard.tsx` |
| Toasts | `src/components/ToastViewport.tsx` |
| Titles, inline errors and review rows | `src/components/PagePrimitives.tsx` |
| Search thumbnails and video preview | `src/features/search/` |
| Tool status/update buttons | `src/features/tools/ToolBadge.tsx` |
| Console line colors and size/speed/ETA parsing | `src/features/downloads/consoleUtils.ts` |
| Format classification | `src/features/downloads/formatUtils.ts` |
| UI state types | `src/features/downloads/uiTypes.ts` |
| History date grouping | `src/features/history/historyUtils.ts` |
| Wizard events, screen composition and state | `src/App.tsx` |
| Windows/Android bridge and DTOs | `src/nativeBridge.ts`, `src/types.ts` |
| yt-dlp command arguments | `src-tauri/src/download/args.rs` |
| Quality selectors and cover host validation | `src-tauri/src/download/formats.rs` |
| Subtitle language/source selection | `src-tauri/src/download/subtitles.rs` |
| Platform URL detection and YouTube HLS fallback | `src-tauri/src/download/sites.rs` |
| Safe output folder names | `src-tauri/src/download/paths.rs` |
| Tool discovery/updates, search and process execution | `src-tauri/src/lib.rs` |
| Spotify track metadata import (Windows; audio selected on YouTube) | `src-tauri/src/spotify.rs`, `src/App.tsx` |
| Android service, storage, FFmpeg and QuickJS | `src-tauri/plugins/mobile-downloader/android/src/main/` |
| Android pause/resume/cancel and active job ownership | `src-tauri/src/android_download/`, called through JNI |
| Standalone Rust Android build and direct-file wizard | `android-rust/` |
| GPU backdrop and Rust WASM math | `src/visuals/gpuBackdrop.ts`, `visual-core/lib.rs` |

## Styles

`src/styles.css` imports the following files **in cascade order**:

1. `tokens.css`: fonts, colors, spacing, radii and cursors.
2. `base.css`: base layout, wizard screens, responsive rules and animations.
3. `components.css`: detailed component treatments, console, history and feedback.
4. `typography.css`: desktop font scale adjustments.
5. `desktop-buttons.css`: Windows button states and interaction details.
6. `desktop-shell.css`: Windows shell, panels and finishing layers.

The cascade was preserved during extraction. Search for the specific selector
before changing it: later files may intentionally override earlier rules.
Cursor URLs are relative to `src/styles/`, not the root stylesheet.

## Quick validation

`npm run check` performs TypeScript validation without bundling.
The visual core requires the Rust target `wasm32-unknown-unknown`. Install it
once with `rustup target add wasm32-unknown-unknown`.
`npm run test:frontend` tests pure helpers without launching the app.
`npm run test:rust` tests native rules using the warm release dependency cache.
`npm run build` and `npm run desktop:exe` use content fingerprints and reuse
unchanged results. Use `--force` only when an actual rebuild is needed.

Frontend tests live in `tests/frontend-utils.test.mjs`; native tests live in
`src-tauri/src/tests.rs`. Add regression tests next to these when changing rules.
