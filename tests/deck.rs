use keyprompter::deck::{Block, SpanStyle, parse};

fn text(spans: &[keyprompter::deck::Span]) -> Vec<(&str, SpanStyle)> {
    spans.iter().map(|s| (s.text.as_str(), s.style)).collect()
}

#[test]
fn section_carries_forward_and_is_not_a_slide() {
    let deck = parse("# A\n## one\n## two\n# B\n## three\n");
    let got: Vec<_> = deck
        .slides
        .iter()
        .map(|s| (s.section.as_deref(), s.cue.as_str()))
        .collect();
    assert_eq!(
        got,
        [(Some("A"), "one"), (Some("A"), "two"), (Some("B"), "three")]
    );
}

#[test]
fn slide_before_any_section_has_none() {
    let deck = parse("## orphan\n# Later\n## kept\n");
    assert_eq!(deck.slides[0].section, None);
    assert_eq!(deck.slides[1].section.as_deref(), Some("Later"));
}

#[test]
fn lines_before_first_heading_are_ignored() {
    let deck = parse("- stray\nnotes\n\n## real\n- b\n");
    assert_eq!(deck.slides.len(), 1);
    assert_eq!(deck.slides[0].body.len(), 1);
}

#[test]
fn body_runs_to_the_next_heading() {
    let deck = parse("## a\n- x\n- y\n## b\n- z\n");
    assert_eq!(deck.slides[0].body.len(), 2);
    assert_eq!(deck.slides[1].body.len(), 1);
}

#[test]
fn bullets_and_paragraphs() {
    let deck = parse("## a\n- dash\n* star\nline one\nline two\n\nsecond para\n");
    let body = &deck.slides[0].body;
    assert!(matches!(&body[0], Block::Bullet(s) if text(s) == [("dash", SpanStyle::Plain)]));
    assert!(matches!(&body[1], Block::Bullet(s) if text(s) == [("star", SpanStyle::Plain)]));
    assert!(
        matches!(&body[2], Block::Para(s) if text(s) == [("line one line two", SpanStyle::Plain)])
    );
    assert!(matches!(&body[3], Block::Para(s) if text(s) == [("second para", SpanStyle::Plain)]));
}

#[test]
fn deep_headings_become_bold_paragraphs() {
    let deck = parse("## a\n### Sub\n- x\n");
    assert!(
        matches!(&deck.slides[0].body[0], Block::Para(s) if text(s) == [("Sub", SpanStyle::Bold)])
    );
    assert_eq!(deck.slides[0].body.len(), 2);
}

#[test]
fn inline_styles() {
    let deck = parse("## a\n- p **b** *i* `c` q\n");
    let Block::Bullet(spans) = &deck.slides[0].body[0] else {
        panic!()
    };
    assert_eq!(
        text(spans),
        [
            ("p ", SpanStyle::Plain),
            ("b", SpanStyle::Bold),
            (" ", SpanStyle::Plain),
            ("i", SpanStyle::Italic),
            (" ", SpanStyle::Plain),
            ("c", SpanStyle::Code),
            (" q", SpanStyle::Plain),
        ]
    );
}

#[test]
fn unmatched_markers_stay_literal() {
    let deck = parse("## a\n- 2 * 3 and `open and **bold\n");
    let Block::Bullet(spans) = &deck.slides[0].body[0] else {
        panic!()
    };
    assert_eq!(
        text(spans),
        [("2 * 3 and `open and **bold", SpanStyle::Plain)]
    );
}

#[test]
fn any_text_yields_a_deck() {
    assert!(parse("").slides.is_empty());
    assert!(parse("just prose\n\n\n").slides.is_empty());
    assert_eq!(parse("##\n").slides[0].cue, "");
}

#[test]
fn example_deck_has_the_expected_cues() {
    let deck = parse(include_str!("../examples/stream.md"));
    let cues: Vec<_> = deck.slides.iter().map(|s| s.cue.as_str()).collect();
    assert_eq!(
        cues,
        [
            "Hook",
            "Who am I",
            "Tokens, not words",
            "Attention",
            "Questions from chat",
            "Wrap up"
        ]
    );
    assert_eq!(deck.slides[2].section.as_deref(), Some("How LLMs read"));
}
