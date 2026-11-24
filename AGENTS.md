# Working on YT-DLP Deck

Read `docs/ARCHITECTURE.md` first; use its feature map to open only relevant files.
Do not read generated binaries, all of `App.tsx`, or the whole stylesheet by default.
Use `rg -n` to locate a symbol, then read a bounded range around it.

## Boundaries

- `src/App.tsx`: wizard orchestration, application state, native events and screen composition.
- `src/components/`: reusable presentation, without download orchestration.
- `src/features/`: feature components and pure helpers.
- `src/config/`: option catalogs and shared motion constants.
- `src/types.ts`: native bridge contracts; keep in sync with Rust/Kotlin.
- `src/styles.css`: ordered imports; edit the relevant file in `src/styles/`.
- `src-tauri/src/lib.rs`: native commands, tool updates and process orchestration.
- `src-tauri/src/download/`: pure download rules, selectors, subtitles and arguments.
- `src-tauri/src/tests.rs`: native regression tests.
- Android engine: `src-tauri/plugins/mobile-downloader/android/src/main/`.

## Verification and builds

- Frontend changes: `npm run check`, `npm run test:frontend`, then `npm run build`.
- Native download rules: `npm run test:rust` (reuses release dependencies).
- Windows executable: `npm run desktop:exe`; installer only when requested: `npm run desktop:build`.
- Android builds and GitHub publishing require a current user request; do not do them automatically.
- Do not delete build caches to troubleshoot unrelated code issues.
- Preserve unrelated working-tree changes. Use patch edits; keep moves behavior-preserving.
- Keep CSS import order and relative asset paths intact. Do not change the GPU backdrop during unrelated refactors.
- Advance only the patch version for updates, syncing package manifests/locks, Cargo and Tauri config.
