//! The AppKit calls GPUI doesn't wrap.

use gpui_kit::Window;
use objc2::rc::Retained;
use objc2_app_kit::{NSEvent, NSView, NSWindow, NSWindowSharingType, NSWorkspace};
use objc2_foundation::{NSPoint, NSRect};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::snap::{self, Rect};

/// `None` for windows without an AppKit view, such as test windows.
fn ns_window(window: &Window) -> Option<Retained<NSWindow>> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI hands out the live NSView backing this window, and we only
    // borrow it for the duration of this call on the main thread.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    view.window()
}

/// Keeps the window out of screenshots, screen sharing and recordings while it
/// stays visible on screen.
pub fn hide_from_capture(window: &Window, hidden: bool) {
    if let Some(ns_window) = ns_window(window) {
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

pub fn mouse_down() -> bool {
    NSEvent::pressedMouseButtons() != 0
}

/// Moves the window onto screen or menu-bar edges within `reach` points.
pub fn snap_to_edges(window: &Window, reach: f32) {
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    let Some(screen) = ns_window.screen() else {
        return;
    };
    let rect = |r: NSRect| Rect {
        x: r.origin.x as f32,
        y: r.origin.y as f32,
        w: r.size.width as f32,
        h: r.size.height as f32,
    };
    if let Some((x, y)) = snap::settle(
        rect(ns_window.frame()),
        rect(screen.frame()),
        rect(screen.visibleFrame()),
        reach,
    ) {
        ns_window.setFrameOrigin(NSPoint::new(x.into(), y.into()));
    }
}
