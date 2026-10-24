//! The AppKit calls GPUI doesn't wrap.

use gpui_kit::Window;
use std::sync::Once;

use objc2::{
    MainThreadMarker,
    ffi::class_addMethod,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Imp, Sel},
    sel,
};
use objc2_app_kit::{NSEvent, NSScreen, NSView, NSWindow, NSWindowSharingType, NSWorkspace};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::{
    snap::{self, Rect},
    state::Frame,
};

fn rect(r: NSRect) -> Rect {
    Rect {
        x: r.origin.x as f32,
        y: r.origin.y as f32,
        w: r.size.width as f32,
        h: r.size.height as f32,
    }
}

fn ns_rect(r: Rect) -> NSRect {
    NSRect::new(
        NSPoint::new(r.x.into(), r.y.into()),
        NSSize::new(r.w.into(), r.h.into()),
    )
}

/// The first screen is the one with the menu bar, whose AppKit origin is (0, 0).
fn screens() -> Vec<Retained<NSScreen>> {
    MainThreadMarker::new().map_or_else(Vec::new, |mtm| NSScreen::screens(mtm).to_vec())
}

/// Lets the window go over the menu bar. GPUI gives its panels a title-bar
/// style, and AppKit's `constrainFrameRect:toScreen:` keeps titled windows
/// below the menu bar. GPUI's panel class doesn't define that method, so we
/// add one that keeps the frame as asked, for placement and drags alike.
pub fn allow_over_menu_bar(window: &Window) {
    extern "C-unwind" fn unconstrained(
        _: &AnyObject,
        _: Sel,
        frame: NSRect,
        _: *mut AnyObject,
    ) -> NSRect {
        frame
    }
    static ADDED: Once = Once::new();
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    ADDED.call_once(|| {
        let class: *const AnyClass = ns_window.class();
        // SAFETY: the signature matches constrainFrameRect:toScreen: and the
        // type string spells it out for a 64-bit CGFloat.
        unsafe {
            let imp: Imp = std::mem::transmute(
                unconstrained
                    as extern "C-unwind" fn(&AnyObject, Sel, NSRect, *mut AnyObject) -> NSRect,
            );
            class_addMethod(
                class.cast_mut(),
                sel!(constrainFrameRect:toScreen:),
                imp,
                c"{CGRect={CGPoint=dd}{CGSize=dd}}@:{CGRect={CGPoint=dd}{CGSize=dd}}@".as_ptr(),
            );
        }
    });
}

pub fn screen_names() -> Vec<String> {
    screens()
        .iter()
        .map(|s| s.localizedName().to_string())
        .collect()
}

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
    if let Some((x, y)) = snap::settle(
        rect(ns_window.frame()),
        rect(screen.frame()),
        rect(screen.visibleFrame()),
        reach,
    ) {
        ns_window.setFrameOrigin(NSPoint::new(x.into(), y.into()));
    }
}

/// Centers the window on the very top edge of a screen, over its menu bar,
/// where macOS won't open or drag a window. `None` means its current screen.
pub fn move_to_top(window: &Window, screen: Option<usize>) {
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    let screen = match screen {
        Some(i) => screens().into_iter().nth(i),
        None => ns_window.screen(),
    };
    if let Some(screen) = screen {
        let placed = snap::top_center(rect(ns_window.frame()), rect(screen.frame()));
        ns_window.setFrame_display(ns_rect(placed), true);
    }
}

/// Puts the window back at a saved frame, given in GPUI's top-left space.
/// A frame on no connected screen lands at the top of the main screen.
pub fn restore(window: &Window, frame: Frame) {
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    let screens = screens();
    let Some(main) = screens.first() else {
        return;
    };
    let main = rect(main.frame());
    let saved = Rect {
        x: frame.x,
        y: main.h - frame.y - frame.h,
        w: frame.w,
        h: frame.h,
    };
    if screens
        .iter()
        .any(|s| snap::overlaps(rect(s.frame()), saved))
    {
        ns_window.setFrame_display(ns_rect(saved), true);
    } else {
        move_to_top(window, Some(0));
    }
}
