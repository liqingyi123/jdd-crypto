use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::Mutex;

pub const DEFAULT_BADGE_SIZE: u32 = 96;

pub struct AppState {
    pub clipboard_watch_enabled: AtomicBool,
    pub last_clipboard: Mutex<String>,
    pub badge_size: AtomicU32,
    pub badge_expanded: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            clipboard_watch_enabled: AtomicBool::new(true),
            last_clipboard: Mutex::new(String::new()),
            badge_size: AtomicU32::new(DEFAULT_BADGE_SIZE),
            badge_expanded: AtomicBool::new(false),
        }
    }
}

pub fn normalize_badge_size(size: u32) -> u32 {
    match size {
        38 | 68 | 96 => size,
        _ => DEFAULT_BADGE_SIZE,
    }
}
