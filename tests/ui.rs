use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions, px,
    size,
};
use keyprompter::view::StageView;

fn open(cx: &mut TestAppContext, deck: &str) -> (AnyWindowHandle, Entity<StageView>) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("kp-ui-{}-{n}.md", std::process::id()));
    std::fs::write(&path, deck).unwrap();
    open_path(cx, path)
}

fn open_path(
    cx: &mut TestAppContext,
    path: std::path::PathBuf,
) -> (AnyWindowHandle, Entity<StageView>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        keyprompter::app::init(cx);
    });
    let opened = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.), px(150.)),
                    cx,
                ))),
                ..WindowOptions::default()
            },
            cx,
            |window, cx| cx.new(|cx| StageView::new(Some(path), None, window, cx)),
        )
        .unwrap()
    });
    cx.run_until_parked();
    opened
}

fn press(cx: &mut TestAppContext, handle: AnyWindowHandle, key: &str) {
    cx.update_window(handle, |_, window, cx| window.press(key, cx))
        .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
async fn keys_reach_the_focused_view_and_move_the_cursor(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n## c\n");
    let cursor = |cx: &mut TestAppContext| cx.read_entity(&view, |v, _| v.cursor());
    assert_eq!(cursor(cx), 0);
    press(cx, handle, "right");
    assert_eq!(cursor(cx), 1);
    press(cx, handle, "space");
    assert_eq!(cursor(cx), 2);
    press(cx, handle, "left");
    assert_eq!(cursor(cx), 1);
    press(cx, handle, "home");
    assert_eq!(cursor(cx), 0);
    press(cx, handle, "end");
    assert_eq!(cursor(cx), 2);
}

#[gpui_kit::test]
async fn unbound_keys_do_nothing(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## only\n## two\n");
    press(cx, handle, "x");
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 0);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.error().map(str::to_owned)),
        None
    );
}

#[gpui_kit::test]
async fn missing_deck_file_surfaces_an_error_and_an_empty_show(cx: &mut TestAppContext) {
    let (handle, view) = open_path(cx, "/nonexistent/kp/deck.md".into());
    press(cx, handle, "right");
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 0);
    assert!(cx.read_entity(&view, |v, _| v.error().is_some()));
}

#[gpui_kit::test]
async fn opening_another_deck_starts_it_from_the_top(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    press(cx, handle, "right");
    let other = std::env::temp_dir().join(format!("kp-ui-{}-other.md", std::process::id()));
    std::fs::write(&other, "## x\n## y\n## z\n").unwrap();
    cx.update(|cx| view.update(cx, |v, cx| v.open(other, cx)));
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 0);
    press(cx, handle, "end");
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 2);
}
