//! The AppKit calls GPUI doesn't wrap.

use gpui_kit::Window;
use objc2_app_kit::{NSView, NSWindowSharingType, NSWorkspace};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Keeps the window out of screenshots, screen sharing and recordings while it
/// stays visible on screen. A no-op for windows without an AppKit view, such as
/// test windows.
pub fn hide_from_capture(window: &Window, hidden: bool) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    // SAFETY: GPUI hands out the live NSView backing this window, and we only
    // borrow it for the duration of this call on the main thread.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    if let Some(ns_window) = view.window() {
        ns_window.setSharingType(if hidden {
            NSWindowSharingType::None
        } else {
            NSWindowSharingType::ReadOnly
        });
    }
}

/// The System Settings > Accessibility > Display > Reduce motion switch.
pub fn reduce_motion() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}
