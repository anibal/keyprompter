use keyprompter::{
    deck::{Span, SpanStyle},
    layout::{CardLayout, Fit, Line, Mark, TypeScale, split, wrap},
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

fn card(cue: usize, body: usize) -> CardLayout {
    CardLayout {
        cue: vec!["cue".into(); cue],
        body: vec![
            Line {
                mark: Mark::Bullet,
                spans: vec![plain("x")],
            };
            body
        ],
    }
}

#[test]
fn the_default_scale_fits_a_cue_and_four_body_lines() {
    let scale = TypeScale::new(118.0, 0);
    assert!((scale.cue / scale.body - 1.4).abs() < 1e-4);
    assert_eq!(
        card(1, 4).fit(scale, 118.0),
        Fit {
            cue: 1,
            body: 4,
            hidden: 0
        }
    );
}

#[test]
fn an_overflowing_card_gives_its_last_row_to_the_badge() {
    let scale = TypeScale::new(118.0, 0);
    assert_eq!(
        card(1, 6).fit(scale, 118.0),
        Fit {
            cue: 1,
            body: 3,
            hidden: 3
        }
    );
}

#[test]
fn a_two_line_cue_takes_room_from_the_body() {
    let scale = TypeScale::new(118.0, 0);
    assert_eq!(
        card(2, 4).fit(scale, 118.0),
        Fit {
            cue: 2,
            body: 1,
            hidden: 3
        }
    );
}

#[test]
fn a_cue_taller_than_the_card_counts_as_hidden() {
    let scale = TypeScale::new(118.0, 0);
    assert_eq!(
        card(4, 1).fit(scale, 118.0),
        Fit {
            cue: 3,
            body: 0,
            hidden: 2
        }
    );
}

fn mono(text: &str, _: SpanStyle) -> f32 {
    text.chars().count() as f32
}

#[test]
fn wrap_breaks_after_spaces_in_reading_order() {
    assert_eq!(wrap(&[plain("aaa bbb ccc")], 7.0, &mut mono), vec![8]);
    assert_eq!(wrap(&[plain("aaa bbb ccc")], 6.0, &mut mono), vec![4, 8]);
}

#[test]
fn wrap_breaks_between_cjk_characters_but_not_before_closing_marks() {
    assert_eq!(
        wrap(&[plain("日本語です。")], 4.0, &mut mono),
        vec!["日本語で".len()]
    );
}

#[test]
fn a_word_wider_than_the_line_breaks_between_characters() {
    assert_eq!(wrap(&[plain("abcdefghij")], 4.0, &mut mono), vec![4, 8]);
}

#[test]
fn wrap_measures_each_style_with_its_own_width() {
    let mut wide_bold = |t: &str, style: SpanStyle| {
        t.chars().count() as f32 * if style == SpanStyle::Bold { 2.0 } else { 1.0 }
    };
    assert_eq!(
        wrap(&[plain("ab "), bold("cd")], 6.0, &mut wide_bold),
        vec![3]
    );
}

#[test]
fn steps_scale_both_sizes_and_stop_at_the_limits() {
    let base = TypeScale::new(118.0, 0);
    let up = TypeScale::new(118.0, 1);
    assert!((up.body / base.body - 1.1).abs() < 1e-4);
    assert_eq!(TypeScale::new(118.0, 99), TypeScale::new(118.0, 8));
}
