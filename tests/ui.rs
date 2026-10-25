use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions, px,
    size,
};
use keyprompter::{
    state::{Capture, State},
    view::StageView,
};

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
            |window, cx| {
                cx.new(|cx| StageView::new(Some(path), State::default(), None, window, cx))
            },
        )
        .unwrap()
    });
    cx.update(|cx| keyprompter::app::connect(opened.0, opened.1.clone(), cx));
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
    cx.update(|cx| keyprompter::app::open(handle, &view, other, cx));
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 0);
    press(cx, handle, "end");
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 2);
}

#[gpui_kit::test]
async fn the_capture_toggle_flips_between_hidden_and_shown(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n");
    let capture = |cx: &mut TestAppContext| cx.read_entity(&view, |v, _| v.capture());
    assert_eq!(capture(cx), Capture::Hidden);
    let toggle = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| {
            view.update(cx, |v, _| v.toggle_capture(window))
        })
        .unwrap()
    };
    toggle(cx);
    assert_eq!(capture(cx), Capture::Shown);
    toggle(cx);
    assert_eq!(capture(cx), Capture::Hidden);
}

#[gpui_kit::test]
async fn app_level_actions_reach_the_strip(cx: &mut TestAppContext) {
    let (_, view) = open(cx, "## a\n## b\n## c\n");
    cx.update(|cx| cx.dispatch_action(&keyprompter::app::Last));
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 2);
}

#[gpui_kit::test]
async fn a_long_bullet_lays_out_as_several_lines_without_losing_words(cx: &mut TestAppContext) {
    let text = "This bullet is long enough that it has to wrap onto more than one line";
    let (handle, _) = open(cx, &format!("## cue\n- {text}\n"));
    let card = cx
        .update_window(handle, |_, window, _| {
            let deck = keyprompter::deck::parse(&format!("## cue\n- {text}\n"));
            let scale = keyprompter::layout::TypeScale::new(118.0, 0);
            keyprompter::layout::layout(&deck.slides[0], scale, 300.0, window)
        })
        .unwrap();
    assert!(card.body.len() > 1, "{card:?}");
    assert_eq!(card.body[0].mark, keyprompter::layout::Mark::Bullet);
    assert!(
        card.body[1..]
            .iter()
            .all(|l| l.mark == keyprompter::layout::Mark::Hang)
    );
    let words: Vec<String> = card
        .body
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.text.clone()))
        .collect();
    assert_eq!(words.join(" "), text);
}

#[gpui_kit::test]
async fn smaller_text_makes_an_overflowing_card_fit(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## cue\n- a\n- b\n- c\n- d\n- e\n## next\n");
    let draw = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        cx.run_until_parked();
    };
    draw(cx);
    assert_eq!(cx.read_entity(&view, |v, _| v.overflowing()), vec![(0, 1)]);
    for _ in 0..3 {
        cx.update(|cx| cx.dispatch_action(&keyprompter::app::SmallerText));
        cx.run_until_parked();
    }
    draw(cx);
    assert_eq!(cx.read_entity(&view, |v, _| v.text_step()), -3);
    assert_eq!(cx.read_entity(&view, |v, _| v.overflowing()), vec![]);
    cx.update(|cx| cx.dispatch_action(&keyprompter::app::DefaultTextSize));
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.text_step()), 0);
}
