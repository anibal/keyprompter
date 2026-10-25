use keyprompter::{
    deck::{Span, SpanStyle},
    layout::{CardLayout, Line, Mark, TypeScale, split},
};

fn plain(text: &str) -> Span {
    Span {
        text: text.into(),
        style: SpanStyle::Plain,
    }
}

fn bold(text: &str) -> Span {
    Span {
        text: text.into(),
        style: SpanStyle::Bold,
    }
}

#[test]
fn split_cuts_inside_and_between_spans_and_trims_the_seams() {
    let spans = [plain("one two "), bold("three four"), plain(" five")];
    assert_eq!(
        split(&spans, &[4, 14]),
        vec![
            vec![plain("one")],
            vec![plain("two "), bold("three")],
            vec![bold("four"), plain(" five")],
        ]
    );
}

#[test]
fn split_without_breaks_is_one_line() {
    assert_eq!(split(&[plain("a b")], &[]), vec![vec![plain("a b")]]);
}

#[test]
fn the_default_scale_fits_a_cue_and_four_body_lines() {
    let scale = TypeScale::new(118.0, 0);
    assert!((scale.cue / scale.body - 1.4).abs() < 1e-4);
    let card = CardLayout {
        cue: vec!["cue".into()],
        body: vec![
            Line {
                mark: Mark::Bullet,
                spans: vec![plain("x")],
            };
            6
        ],
    };
    assert_eq!(card.visible_body(scale, 118.0), 4);
    assert_eq!(card.hidden_body(scale, 118.0), 2);
}

#[test]
fn a_two_line_cue_takes_room_from_the_body() {
    let scale = TypeScale::new(118.0, 0);
    let card = CardLayout {
        cue: vec!["a".into(), "b".into()],
        body: vec![
            Line {
                mark: Mark::None,
                spans: vec![plain("x")],
            };
            4
        ],
    };
    assert_eq!(card.visible_body(scale, 118.0), 2);
}

#[test]
fn steps_scale_both_sizes_and_stop_at_the_limits() {
    let base = TypeScale::new(118.0, 0);
    let up = TypeScale::new(118.0, 1);
    assert!((up.body / base.body - 1.1).abs() < 1e-4);
    assert_eq!(TypeScale::new(118.0, 99), TypeScale::new(118.0, 8));
}
