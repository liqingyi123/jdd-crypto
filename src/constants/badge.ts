/**
 * Badge geometry constants shared by the frontend.
 * Keep in sync with `src-tauri/src/state.rs` (`DEFAULT_BADGE_SIZE`) and
 * `src-tauri/src/windows.rs` (`BADGE_EDGE_MARGIN`).
 */
export const DEFAULT_BADGE_SIZE = 68;
/** Hide badge window when selected in settings. Keep in sync with `state.rs` (`BADGE_HIDDEN_SIZE`). */
export const BADGE_HIDDEN_SIZE = 0;
/** Default distance from screen work-area edges (logical px). Keep in sync with `windows.rs`. */
export const BADGE_EDGE_MARGIN = 50;
