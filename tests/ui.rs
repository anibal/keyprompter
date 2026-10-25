use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, TestAppContext, WindowBounds, WindowOptions, px,
    size,
};
use keyprompter::{
    app::open_markdown,
    document::Document,
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
    let document = Some(Document::new("test".into(), Rundown::import(deck, "test")));
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
                    StageView::new(
                        document,
                        library.clone(),
                        State::default(),
                        None,
                        window,
                        cx,
                    )
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
            view.update(cx, |v, cx| v.toggle_capture(window, cx))
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
        v.document()
            .unwrap()
            .rundown()
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
    use keyprompter::app::{EditCard, Redo, Undo};
    let library = library();
    let (handle, view) = open_in(cx, "## a\n## b\n## c\n", library.clone());
    press(cx, handle, "right");

    act(cx, EditCard::Duplicate);
    assert_eq!(cues(cx, &view), ["a", "b", "b", "c"]);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.cursor()),
        2,
        "focus moves to the copy"
    );
    assert_eq!(library.load("test").unwrap().card_ids().len(), 4, "saved");

    act(cx, EditCard::MoveAfter);
    act(cx, EditCard::Delete);
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

#[gpui_kit::test]
async fn cmd_z_on_the_strip_undoes_and_a_failed_save_error_clears_on_the_next_save(
    cx: &mut TestAppContext,
) {
    let (handle, view) = open(cx, "## a\n## b\n");
    act(cx, keyprompter::app::EditCard::Duplicate);
    assert_eq!(cues(cx, &view), ["a", "a", "b"]);
    press(cx, handle, "cmd-z");
    assert_eq!(cues(cx, &view), ["a", "b"]);
    press(cx, handle, "shift-cmd-z");
    assert_eq!(cues(cx, &view), ["a", "a", "b"]);
    assert_eq!(
        cx.read_entity(&view, |v, _| v.error().map(str::to_owned)),
        None
    );
}

#[gpui_kit::test]
async fn undo_never_hands_out_a_discarded_card_id_again(cx: &mut TestAppContext) {
    let (_, view) = open(cx, "## a\n");
    let ids = |cx: &mut TestAppContext| {
        cx.read_entity(&view, |v, _| v.document().unwrap().rundown().card_ids())
    };
    act(cx, keyprompter::app::EditCard::Duplicate);
    let first_copy = ids(cx)[1];
    act(cx, keyprompter::app::Undo);
    act(cx, keyprompter::app::EditCard::Duplicate);
    assert_ne!(ids(cx)[1], first_copy);
}

#[gpui_kit::test]
async fn right_clicking_a_card_leaves_the_hot_spot_where_it_is(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n## c\n");
    let mut cx = gpui_kit::VisualTestContext::from_window(handle, cx);
    let next_card = gpui_kit::point(px(942.), px(75.));
    cx.simulate_mouse_down(next_card, gpui_kit::MouseButton::Right, Default::default());
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 0);
}

#[gpui_kit::test]
async fn clicking_a_card_brings_it_to_the_middle(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n## c\n");
    let mut cx = gpui_kit::VisualTestContext::from_window(handle, cx);
    let next_card = gpui_kit::point(px(942.), px(75.));
    cx.simulate_click(next_card, Default::default());
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 1);
}

#[gpui_kit::test]
async fn undo_and_redo_name_the_change_and_gray_out_when_empty(cx: &mut TestAppContext) {
    use gpui_kit::OwnedMenuItem;
    let (_, _view) = open(cx, "## a\n## b\n");
    let edit_menu = |cx: &mut TestAppContext| -> Vec<(String, bool)> {
        cx.update(|cx| {
            let menus = cx.get_menus().unwrap();
            let edit = menus.iter().find(|m| m.name == "Edit").unwrap();
            edit.items
                .iter()
                .filter_map(|item| match item {
                    OwnedMenuItem::Action { name, disabled, .. } => Some((name.clone(), *disabled)),
                    _ => None,
                })
                .collect()
        })
    };
    let item = |name: &str, disabled| (name.to_string(), disabled);
    assert_eq!(edit_menu(cx), [item("Undo", true), item("Redo", true)]);
    act(cx, keyprompter::app::EditCard::Delete);
    assert_eq!(
        edit_menu(cx),
        [item("Undo Delete Card", false), item("Redo", true)]
    );
    act(cx, keyprompter::app::Undo);
    assert_eq!(
        edit_menu(cx),
        [item("Undo", true), item("Redo Delete Card", false)]
    );
}

#[test]
fn the_view_menu_checks_the_side_the_strip_is_stuck_to() {
    use gpui_kit::MenuItem;
    use keyprompter::{
        app::{UndoState, menus},
        snap::Side,
    };
    let sticks = |stuck: Option<Side>| -> Vec<(String, bool)> {
        let menus = menus(Capture::Hidden, &[], &[], UndoState::default(), stuck);
        let view = menus.iter().find(|m| m.name == "View").unwrap();
        view.items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action { name, checked, .. } if name.starts_with("Stick to") => {
                    Some((name.to_string(), *checked))
                }
                _ => None,
            })
            .collect()
    };
    let item = |name: &str, checked| (name.to_string(), checked);
    assert_eq!(
        sticks(None),
        [
            item("Stick to Left", false),
            item("Stick to Center", false),
            item("Stick to Right", false)
        ]
    );
    assert_eq!(
        sticks(Some(Side::Right)),
        [
            item("Stick to Left", false),
            item("Stick to Center", false),
            item("Stick to Right", true)
        ]
    );
}

fn card_text(cx: &mut TestAppContext, view: &Entity<StageView>, index: usize) -> (String, String) {
    cx.read_entity(view, |v, _| {
        let card = v.document().unwrap().rundown().cards().nth(index).unwrap();
        (card.cue.clone(), card.body.clone())
    })
}

fn editing(cx: &mut TestAppContext, view: &Entity<StageView>) -> bool {
    cx.read_entity(view, |v, _| v.editing())
}

fn strings(cue: &str, body: &str) -> (String, String) {
    (cue.to_string(), body.to_string())
}

#[gpui_kit::test]
async fn return_opens_the_card_and_cmd_return_saves_what_was_typed(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\nold\n## b\n");
    press(cx, handle, "enter");
    assert!(editing(cx, &view));
    cx.simulate_keystrokes(handle, "x space y");
    cx.simulate_keystrokes(handle, "enter");
    cx.simulate_keystrokes(handle, "n e w space t e x t");
    cx.simulate_keystrokes(handle, "cmd-enter");
    assert!(!editing(cx, &view));
    assert_eq!(card_text(cx, &view, 0), strings("ax y", "new textold"));
    assert_eq!(
        cx.read_entity(&view, |v, _| v.cursor()),
        0,
        "a space typed in a field is not Next"
    );
    press(cx, handle, "right");
    assert_eq!(
        cx.read_entity(&view, |v, _| v.cursor()),
        1,
        "keys navigate again"
    );
}

#[gpui_kit::test]
async fn escape_discards_the_edit(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    act(cx, keyprompter::app::EditText);
    cx.simulate_keystrokes(handle, "z");
    press(cx, handle, "escape");
    assert!(!editing(cx, &view));
    assert_eq!(card_text(cx, &view, 0), strings("a", ""));
    act(cx, keyprompter::app::Undo);
    assert_eq!(card_text(cx, &view, 0), strings("a", ""));
}

#[gpui_kit::test]
async fn a_saved_edit_is_one_undo_step_named_edit_card(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\nline\n## b\n");
    press(cx, handle, "enter");
    cx.simulate_keystrokes(handle, "b a c k space e n d");
    cx.simulate_keystrokes(handle, "enter");
    cx.simulate_keystrokes(handle, "x");
    cx.simulate_keystrokes(handle, "cmd-enter");
    assert_eq!(card_text(cx, &view, 0), strings("aback end", "xline"));
    let label = cx.read_entity(&view, |v, _| v.document().unwrap().undo_label());
    assert_eq!(label, Some("Edit Card"));
    act(cx, keyprompter::app::Undo);
    assert_eq!(card_text(cx, &view, 0), strings("a", "line"));
    act(cx, keyprompter::app::Redo);
    assert_eq!(card_text(cx, &view, 0), strings("aback end", "xline"));
}

#[gpui_kit::test]
async fn an_unchanged_or_empty_cue_records_no_step(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    press(cx, handle, "enter");
    press(cx, handle, "cmd-enter");
    let undo =
        |cx: &mut TestAppContext| cx.read_entity(&view, |v, _| v.document().unwrap().undo_label());
    assert_eq!(undo(cx), None, "nothing changed");
    press(cx, handle, "enter");
    press(cx, handle, "backspace");
    press(cx, handle, "cmd-enter");
    assert!(!editing(cx, &view));
    assert_eq!(
        card_text(cx, &view, 0),
        strings("a", ""),
        "an empty cue reverts"
    );
    assert_eq!(undo(cx), None);
}

#[gpui_kit::test]
async fn moving_the_strip_saves_the_edit_first(cx: &mut TestAppContext) {
    let library = library();
    let (handle, view) = open_in(cx, "## a\n## b\n", library.clone());
    press(cx, handle, "enter");
    cx.simulate_keystrokes(handle, "x");
    act(cx, keyprompter::app::Next);
    assert!(!editing(cx, &view));
    assert_eq!(card_text(cx, &view, 0), strings("ax", ""));
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 1);
    assert_eq!(
        library.load("test").unwrap().cards().next().unwrap().cue,
        "ax",
        "saved"
    );
}

#[gpui_kit::test]
async fn a_card_operation_saves_the_edit_first(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    press(cx, handle, "enter");
    cx.simulate_keystrokes(handle, "x");
    act(cx, keyprompter::app::EditCard::Duplicate);
    assert!(!editing(cx, &view));
    assert_eq!(cues(cx, &view), ["ax", "ax", "b"]);
}

#[gpui_kit::test]
async fn undo_while_editing_goes_to_the_field_not_the_cards(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    act(cx, keyprompter::app::EditCard::Duplicate);
    act(cx, keyprompter::app::EditText);
    cx.simulate_keystrokes(handle, "x y");
    act(cx, keyprompter::app::Undo);
    assert_eq!(cues(cx, &view), ["a", "a", "b"], "the duplicate stays");
    assert!(editing(cx, &view));
    press(cx, handle, "cmd-enter");
    assert_eq!(cues(cx, &view), ["a", "a", "b"]);
}

#[gpui_kit::test]
async fn double_clicking_the_current_card_edits_it_and_another_card_only_centers(
    cx: &mut TestAppContext,
) {
    let (handle, view) = open(cx, "## a\n## b\n## c\n");
    let mut cx = gpui_kit::VisualTestContext::from_window(handle, cx);
    let next_card = gpui_kit::point(px(942.), px(75.));
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position: next_card,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 2,
        first_mouse: false,
    });
    cx.run_until_parked();
    assert_eq!(cx.read_entity(&view, |v, _| v.cursor()), 1);
    assert!(!cx.read_entity(&view, |v, _| v.editing()));
    cx.simulate_event(gpui_kit::MouseDownEvent {
        position: next_card,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 2,
        first_mouse: false,
    });
    cx.run_until_parked();
    assert!(cx.read_entity(&view, |v, _| v.editing()));
}

#[gpui_kit::test]
async fn arrows_cross_between_the_fields_and_return_in_the_body_adds_a_line(
    cx: &mut TestAppContext,
) {
    let (handle, view) = open(cx, "## a\nl1\n## b\n");
    press(cx, handle, "enter");
    press(cx, handle, "down");
    cx.simulate_keystrokes(handle, "x enter y");
    assert!(editing(cx, &view), "plain Return in the body does not save");
    press(cx, handle, "up");
    press(cx, handle, "up");
    cx.simulate_keystrokes(handle, "z");
    press(cx, handle, "cmd-enter");
    assert_eq!(card_text(cx, &view, 0), strings("az", "x\nyl1"));
}

#[gpui_kit::test]
async fn pressing_inside_the_fields_keeps_editing(cx: &mut TestAppContext) {
    let (handle, view) = open(cx, "## a\n## b\n");
    press(cx, handle, "enter");
    let mut cx = gpui_kit::VisualTestContext::from_window(handle, cx);
    cx.simulate_click(gpui_kit::point(px(450.), px(30.)), Default::default());
    cx.simulate_click(gpui_kit::point(px(450.), px(100.)), Default::default());
    cx.run_until_parked();
    assert!(cx.read_entity(&view, |v, _| v.editing()));
}
