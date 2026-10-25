//! Layout against the real CoreText shaper; the default test text system
//! measures nothing.

use std::sync::Arc;

use gpui_kit::{
    AppContext, Bounds, TestAppContext, TestDispatcher, WindowBounds, WindowOptions, px, size,
};
use keyprompter::{
    deck::parse,
    layout::{CardLayout, Mark, TypeScale, layout},
    state::State,
    view::StageView,
};

fn lay_out(markdown: &str, width: f32) -> CardLayout {
    let mut cx = TestAppContext::build_with_text_system(
        TestDispatcher::new(0),
        None,
        Arc::new(gpui_apple::AppleTextSystem::new()),
    );
    cx.update(gpui_kit::init);
    let (handle, _) = cx.update(|cx| {
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
            |window, cx| cx.new(|cx| StageView::new(None, State::default(), None, window, cx)),
        )
        .unwrap()
    });
    let deck = parse(markdown);
    cx.update_window(handle, |_, window, _| {
        layout(&deck.slides[0], TypeScale::new(118.0, 0), width, window)
    })
    .unwrap()
}

fn words(card: &CardLayout) -> String {
    let lines: Vec<String> = card
        .body
        .iter()
        .map(|l| l.spans.iter().map(|s| s.text.as_str()).collect())
        .collect();
    lines.join(" ")
}

#[test]
fn a_long_bullet_wraps_into_hanging_lines_without_losing_words() {
    let text = "This bullet is long enough that it has to wrap onto more than one line";
    let card = lay_out(&format!("## cue\n- {text}\n"), 300.0);
    assert!(card.body.len() > 1, "{card:?}");
    assert_eq!(card.body[0].mark, Mark::Bullet);
    assert!(card.body[1..].iter().all(|l| l.mark == Mark::Hang));
    assert_eq!(words(&card), text);
}

#[test]
fn right_to_left_text_wraps_without_panicking_or_losing_words() {
    for text in [
        "שלום עולם זה משפט ארוך מספיק כדי לעטוף לשורה נוספת בכרטיס הזה",
        "مرحبا بالعالم هذه جملة طويلة بما يكفي لتلتف إلى سطر آخر في البطاقة",
        "Mixed עברית and English text that wraps across the card width",
    ] {
        let card = lay_out(&format!("## cue\n- {text}\n"), 300.0);
        assert!(card.body.len() > 1, "{text}: {card:?}");
        assert!(
            card.body.iter().all(|l| !l.spans.is_empty()),
            "{text}: {card:?}"
        );
        let mut got: Vec<&str> = text.split_whitespace().collect();
        let joined = words(&card);
        let mut have: Vec<&str> = joined.split_whitespace().collect();
        got.sort_unstable();
        have.sort_unstable();
        assert_eq!(have, got, "{text}");
    }
}
