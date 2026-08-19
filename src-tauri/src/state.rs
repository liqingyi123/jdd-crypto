use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

pub const MIN_BADGE_SIZE: u32 = 38;
/// Default badge diameter in CSS/logical pixels. Keep in sync with `src/constants/badge.ts`.
pub const DEFAULT_BADGE_SIZE: u32 = 68;
pub const DEFAULT_MOUSE_FOLLOW_SHORTCUT: &str = "Ctrl+Shift+G";

pub struct AppState {
    pub clipboard_watch_enabled: AtomicBool,
    pub last_clipboard: Mutex<String>,
    pub badge_size: AtomicU32,
    pub badge_expanded: AtomicBool,
    pub mouse_follow_enabled: AtomicBool,
    pub mouse_follow_pref_enabled: AtomicBool,
    pub mouse_follow_returning: AtomicBool,
    pub saved_badge_pos: Mutex<Option<(i32, i32)>>,
    pub mouse_follow_shortcut: Mutex<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            clipboard_watch_enabled: AtomicBool::new(true),
            last_clipboard: Mutex::new(String::new()),
            badge_size: AtomicU32::new(DEFAULT_BADGE_SIZE),
            badge_expanded: AtomicBool::new(false),
            mouse_follow_enabled: AtomicBool::new(false),
            mouse_follow_pref_enabled: AtomicBool::new(true),
            mouse_follow_returning: AtomicBool::new(false),
            saved_badge_pos: Mutex::new(None),
            mouse_follow_shortcut: Mutex::new(DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string()),
        }
    }
}

impl AppState {
    pub fn follow_blocks_resize(&self) -> bool {
        self.mouse_follow_enabled.load(Ordering::Relaxed)
            || self.mouse_follow_returning.load(Ordering::Relaxed)
    }
}

pub fn normalize_badge_size(size: u32) -> u32 {
    match size {
        MIN_BADGE_SIZE | 68 | 96 => size,
        _ => DEFAULT_BADGE_SIZE,
    }
}
