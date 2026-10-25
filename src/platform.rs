//! The AppKit calls GPUI doesn't wrap.

use gpui_kit::Window;
use std::{cell::Cell, sync::Once};

use objc2::{
    MainThreadMarker, MainThreadOnly, Message, define_class,
    ffi::class_addMethod,
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject, Imp, Sel},
    sel,
};
use objc2_app_kit::{
    NSEvent, NSMenu, NSMenuItem, NSScreen, NSView, NSWindow, NSWindowSharingType, NSWorkspace,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
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
            let added = class_addMethod(
                class.cast_mut(),
                sel!(constrainFrameRect:toScreen:),
                imp,
                c"{CGRect={CGPoint=dd}{CGSize=dd}}@:{CGRect={CGPoint=dd}{CGSize=dd}}@".as_ptr(),
            );
            if !added.as_bool() {
                eprintln!(
                    "GPUI's panel already constrains frames; the strip stays below the menu bar"
                );
            }
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
        None => ns_window.screen().or_else(|| screens().into_iter().next()),
    };
    if let Some(screen) = screen {
        let placed = snap::top_center(rect(ns_window.frame()), rect(screen.frame()));
        ns_window.setFrame_display(ns_rect(placed), true);
    }
}

/// The window's frame in one space shared by every screen: top-left origin at
/// the main screen's top-left corner, y down. GPUI's own bounds are relative
/// to the window's current screen, which can't say which screen that is.
pub fn frame(window: &Window) -> Option<Frame> {
    let ns_window = ns_window(window)?;
    let main = rect(screens().first()?.frame());
    let f = rect(ns_window.frame());
    Some(Frame {
        x: f.x,
        y: main.h - f.y - f.h,
        w: f.w,
        h: f.h,
    })
}

/// Puts the window back at a frame from [`frame`].
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

thread_local! {
    static CHOSEN: Cell<Option<isize>> = const { Cell::new(None) };
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and MenuTarget has no Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(choose:))]
        fn choose(&self, item: &NSMenuItem) {
            CHOSEN.set(Some(item.tag()));
        }
    }

    unsafe impl NSObjectProtocol for MenuTarget {}
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        // SAFETY: NSObject's init on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }
}

pub fn ns_view(window: &Window) -> Option<Retained<NSView>> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI's live NSView for this window; retaining keeps it valid.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    Some(view.retain())
}

/// Shows a native menu at `at` (window points from the top-left) and waits
/// for a choice. `None` items are separators. The menu runs its own event
/// loop, so call this outside any GPUI update, or GPUI work that runs while
/// the menu is open would find the app already borrowed.
pub fn context_menu(view: &NSView, items: &[Option<&str>], at: (f32, f32)) -> Option<usize> {
    let mtm = MainThreadMarker::new()?;
    let target = MenuTarget::new(mtm);
    let menu = NSMenu::new(mtm);
    for (i, item) in items.iter().enumerate() {
        let entry = match item {
            None => NSMenuItem::separatorItem(mtm),
            Some(title) => {
                // SAFETY: choose: is defined on MenuTarget, which outlives the menu.
                let entry = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(title),
                        Some(sel!(choose:)),
                        &NSString::new(),
                    )
                };
                // SAFETY: the target is retained until this function returns.
                unsafe { entry.setTarget(Some(&target)) };
                entry.setTag(i as isize);
                entry
            }
        };
        menu.addItem(&entry);
    }
    let y = if view.isFlipped() {
        at.1
    } else {
        view.frame().size.height as f32 - at.1
    };
    CHOSEN.set(None);
    menu.popUpMenuPositioningItem_atLocation_inView(
        None,
        NSPoint::new(at.0.into(), y.into()),
        Some(view),
    );
    CHOSEN.take().map(|i| i as usize)
}
