use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use serde::Serialize;

use crate::app_update::UpdateCheckResult;

pub const BADGE_HIDDEN_SIZE: u32 = 0;
pub const MIN_BADGE_SIZE: u32 = 38;
/// Default badge diameter in CSS/logical pixels. Keep in sync with `src/constants/badge.ts`.
pub const DEFAULT_BADGE_SIZE: u32 = 68;
pub const DEFAULT_MOUSE_FOLLOW_SHORTCUT: &str = "Ctrl+Shift+G";
pub const DEFAULT_COMPARE_MODE_SHORTCUT: &str = "Ctrl+Shift+D";

#[derive(Clone, Serialize)]
pub struct ClipboardCandidate {
    pub text: String,
    pub kind: String,
}

pub struct AppState {
    pub clipboard_watch_enabled: AtomicBool,
    pub last_clipboard: Mutex<String>,
    pub last_candidate: Mutex<Option<ClipboardCandidate>>,
    pub badge_size: AtomicU32,
    pub badge_expanded: AtomicBool,
    pub mouse_follow_enabled: AtomicBool,
    pub mouse_follow_pref_enabled: AtomicBool,
    pub mouse_follow_returning: AtomicBool,
    pub saved_badge_pos: Mutex<Option<(i32, i32)>>,
    pub mouse_follow_shortcut: Mutex<String>,
    pub compare_pref_enabled: AtomicBool,
    pub compare_active: AtomicBool,
    pub compare_mode_shortcut: Mutex<String>,
    pub pending_update: Mutex<Option<UpdateCheckResult>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            clipboard_watch_enabled: AtomicBool::new(true),
            last_clipboard: Mutex::new(String::new()),
            last_candidate: Mutex::new(None),
            badge_size: AtomicU32::new(DEFAULT_BADGE_SIZE),
            badge_expanded: AtomicBool::new(false),
            mouse_follow_enabled: AtomicBool::new(false),
            mouse_follow_pref_enabled: AtomicBool::new(true),
            mouse_follow_returning: AtomicBool::new(false),
            saved_badge_pos: Mutex::new(None),
            mouse_follow_shortcut: Mutex::new(DEFAULT_MOUSE_FOLLOW_SHORTCUT.to_string()),
            compare_pref_enabled: AtomicBool::new(true),
            compare_active: AtomicBool::new(false),
            compare_mode_shortcut: Mutex::new(DEFAULT_COMPARE_MODE_SHORTCUT.to_string()),
            pending_update: Mutex::new(None),
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
        BADGE_HIDDEN_SIZE | MIN_BADGE_SIZE | 68 | 96 => size,
        _ => DEFAULT_BADGE_SIZE,
    }
}
