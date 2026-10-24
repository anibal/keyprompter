use gpui_kit::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

struct Spike {
    focus: FocusHandle,
    last: SharedString,
}

impl Render for Spike {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, e: &KeyDownEvent, _, cx| {
                eprintln!("[spike] key {}", e.keystroke.key);
                this.last = e.keystroke.key.clone().into();
                cx.notify();
            }))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_3xl()
            .text_color(rgb(0xffffff))
            .bg(rgba(0x00000088))
            .child(format!("SPIKE last key: {}", self.last))
    }
}

fn keep_on_deactivate(window: &Window) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return };
    let view: &objc2_app_kit::NSView = unsafe { h.ns_view.cast().as_ref() };
    if let Some(ns_window) = view.window() {
        if let Some(panel) = ns_window.downcast_ref::<objc2_app_kit::NSPanel>() {
            eprintln!("[spike] hidesOnDeactivate was {}", panel.hidesOnDeactivate());
        }
        ns_window.setHidesOnDeactivate(false);
    }
}

fn main() {
    let kind = match std::env::var("KP_KIND").as_deref() {
        Ok("floating") => WindowKind::Floating,
        Ok("normal") => WindowKind::Normal,
        _ => WindowKind::PopUp,
    };
    let fix = std::env::var("KP_FIX").is_ok();
    application().run(move |cx| {
        init(cx);
        open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(200.), px(std::env::var("KP_Y").ok().and_then(|v| v.parse().ok()).unwrap_or(0.))),
                    size: size(px(900.), px(140.)),
                })),
                titlebar: None,
                kind,
                is_movable: true,
                window_background: WindowBackgroundAppearance::Blurred,
                ..WindowOptions::default()
            },
            cx,
            |window, cx| {
                if fix {
                    keep_on_deactivate(window);
                }
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Spike { focus, last: "-".into() }
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
