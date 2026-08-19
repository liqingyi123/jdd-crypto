/**
 * Badge geometry constants shared by the frontend.
 * Keep in sync with `src-tauri/src/state.rs` (`DEFAULT_BADGE_SIZE`) and
 * `src-tauri/src/windows.rs` (`EXPANDED_EXTRA_WIDTH` / `EXPANDED_EXTRA_HEIGHT`).
 *
 * Two writers resize the badge window:
 * - Rust `windows::set_badge_size` / `apply_badge_window_size` (prefs + expand flag)
 * - Frontend `useBadgePromptPlacement` via `WebviewWindow.setSize` during prompt expand
 * Prefer Rust for persisted size; frontend only adjusts while the prompt is open.
 */
export const DEFAULT_BADGE_SIZE = 68;
export const EXPANDED_EXTRA_WIDTH = 188;
export const EXPANDED_EXTRA_HEIGHT = 116;
/** Default distance from screen work-area edges (logical px). Keep in sync with `windows.rs`. */
export const BADGE_EDGE_MARGIN = 50;
