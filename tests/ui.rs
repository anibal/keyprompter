use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions, px,
    size,
};
use keyprompter::{
    app::open_markdown,
    library::Library,
    rundown::Rundown,
    state::{Capture, State},
    view::StageView,
};

fn library() -> Library {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Library::new(std::env::temp_dir().join(format!("kp-ui-{}-{n}", std::process::id())))
}

fn open(cx: &mut TestAppContext, deck: &str) -> (AnyWindowHandle, Entity<StageView>) {
    open_in(cx, deck, library())
}

fn open_in(
    cx: &mut TestAppContext,
    deck: &str,
    library: Library,
) -> (AnyWindowHandle, Entity<StageView>) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        keyprompter::app::init(cx);
    });
    let rundown = Some(("test".to_string(), Rundown::import(deck, "test")));
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
                cx.new(|cx| {
                    StageView::new(rundown, library.clone(), State::default(), None, window, cx)
                })
            },
        )
        .unwrap()
    });
    cx.update(|cx| keyprompter::app::connect(opened.0, opened.1.clone(), library, cx));
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
async fn a_missing_markdown_file_shows_an_error_and_keeps_the_current_rundown(
    cx: &mut TestAppContext,
) {
    let (handle, view) = open(cx, "## a\n## b\n");
    cx.update(|cx| {
        open_markdown(
            handle,
            &view,
            &library(),
            "/nonexistent/kp/deck.md".as_ref(),
            cx,
        )
    });
    cx.run_until_parked();
    assert!(cx.read_entity(&view, |v, _| v.error().is_some()));
    press(cx, handle, "end");
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 1);
}

#[gpui_kit::test]
async fn opening_another_deck_starts_it_from_the_top(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    press(cx, handle, "right");
    let other = std::env::temp_dir().join(format!("kp-ui-{}-other.md", std::process::id()));
    std::fs::write(&other, "## x\n## y\n## z\n").unwrap();
    cx.update(|cx| open_markdown(handle, &view, &library(), &other, cx));
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
async fn smaller_text_makes_an_overflowing_card_fit(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## cue\n- a\n- b\n- c\n- d\n- e\n## next\n");
    let draw = |cx: &mut TestAppContext| {
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        cx.run_until_parked();
    };
    draw(cx);
    assert_eq!(cx.read_entity(&view, |v, _| v.overflowing()), vec![(0, 2)]);
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

fn cues(cx: &mut TestAppContext, view: &Entity<StageView>) -> Vec<String> {
    cx.read_entity(view, |v, _| {
        v.rundown()
            .unwrap()
            .1
            .sections
            .iter()
            .flat_map(|s| s.cards.iter().map(|c| c.cue.clone()))
            .collect()
    })
}

fn act(cx: &mut TestAppContext, action: impl gpui_kit::Action) {
    cx.update(|cx| cx.dispatch_action(&action));
    cx.run_until_parked();
}

#[gpui_kit::test]
async fn card_menu_edits_act_on_the_current_card_save_and_undo(cx: &mut TestAppContext) {
    use keyprompter::app::{DeleteCard, DuplicateCard, MoveCardLater, Redo, Undo};
    let library = library();
    let (handle, view) = open_in(cx, "## a\n## b\n## c\n", library.clone());
    press(cx, handle, "right");

    act(cx, DuplicateCard);
    assert_eq!(cues(cx, &view), ["a", "b", "b", "c"]);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.cursor()),
        2,
        "focus moves to the copy"
    );
    assert_eq!(library.load("test").unwrap().card_ids().len(), 4, "saved");

    act(cx, MoveCardLater);
    act(cx, DeleteCard);
    assert_eq!(cues(cx, &view), ["a", "b", "c"]);

    act(cx, Undo);
    assert_eq!(cues(cx, &view), ["a", "b", "c", "b"]);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.cursor()),
        3,
        "undo returns to the deleted card"
    );
    act(cx, Undo);
    assert_eq!(cues(cx, &view), ["a", "b", "b", "c"]);
    act(cx, Undo);
    assert_eq!(cues(cx, &view), ["a", "b", "c"]);
    act(cx, Redo);
    assert_eq!(cues(cx, &view), ["a", "b", "b", "c"]);
    assert_eq!(
        library.load("test").unwrap().card_ids().len(),
        4,
        "undo and redo save too"
    );
}
