use keyprompter::{
    document::Document,
    rundown::{CardId, Edit, Rundown},
};

const SHOW: &str = "## Open\nhello\n# A\n## One\n- one\n## Two\n# B\n## Three\nthree\n";

fn layout(r: &Rundown) -> Vec<(Option<String>, Vec<String>)> {
    r.sections
        .iter()
        .map(|s| {
            (
                s.title.clone(),
                s.cards.iter().map(|c| c.cue.clone()).collect(),
            )
        })
        .collect()
}

fn id(r: &Rundown, cue: &str) -> CardId {
    r.sections
        .iter()
        .flat_map(|s| &s.cards)
        .find(|c| c.cue == cue)
        .unwrap()
        .id
}

fn sections(spec: &[(Option<&str>, &[&str])]) -> Vec<(Option<String>, Vec<String>)> {
    spec.iter()
        .map(|(t, cues)| {
            (
                t.map(String::from),
                cues.iter().map(|c| c.to_string()).collect(),
            )
        })
        .collect()
}

#[test]
fn duplicate_puts_a_copy_with_a_new_id_right_after_and_focuses_it() {
    let mut r = Rundown::import(SHOW, "s");
    let one = id(&r, "One");
    let copy = r.apply(Edit::Duplicate(one)).unwrap();
    assert_ne!(copy, one);
    assert_eq!(
        layout(&r),
        sections(&[
            (None, &["Open"]),
            (Some("A"), &["One", "One", "Two"]),
            (Some("B"), &["Three"])
        ])
    );
    assert_eq!(r.card_ids()[2], copy);
}

#[test]
fn merge_turns_the_next_cue_into_a_bullet_and_keeps_its_body() {
    let mut r = Rundown::import(SHOW, "s");
    let two = id(&r, "Two");
    assert_eq!(r.apply(Edit::MergeWithNext(two)), Some(two));
    let merged = r.sections[1].cards.last().unwrap();
    assert_eq!(merged.body, "- Three\nthree");
    assert_eq!(
        layout(&r),
        sections(&[(None, &["Open"]), (Some("A"), &["One", "Two"])]),
        "B lost its only card, so it is gone"
    );
}

#[test]
fn moving_past_a_section_edge_joins_the_neighbouring_section() {
    let mut r = Rundown::import(SHOW, "s");
    let one = id(&r, "One");
    r.apply(Edit::MoveBefore(one)).unwrap();
    assert_eq!(
        layout(&r),
        sections(&[
            (None, &["Open", "One"]),
            (Some("A"), &["Two"]),
            (Some("B"), &["Three"])
        ])
    );
    r.apply(Edit::MoveAfter(one)).unwrap();
    assert_eq!(
        layout(&r),
        sections(&[
            (None, &["Open"]),
            (Some("A"), &["One", "Two"]),
            (Some("B"), &["Three"])
        ])
    );
}

#[test]
fn an_emptied_section_disappears_so_only_the_first_can_be_untitled() {
    let mut r = Rundown::import(SHOW, "s");
    r.apply(Edit::MoveAfter(id(&r, "Open"))).unwrap();
    assert_eq!(
        layout(&r),
        sections(&[
            (Some("A"), &["Open", "One", "Two"]),
            (Some("B"), &["Three"])
        ])
    );
    assert_eq!(Rundown::import(&r.export(), "s").sections.len(), 2);
}

#[test]
fn delete_focuses_the_next_card_or_the_previous_at_the_end() {
    let mut r = Rundown::import(SHOW, "s");
    let two = id(&r, "Two");
    let three = id(&r, "Three");
    assert_eq!(r.apply(Edit::Delete(two)), Some(three));
    assert_eq!(r.apply(Edit::Delete(three)), Some(id(&r, "One")));
}

#[test]
fn edits_that_cannot_apply_change_nothing() {
    let r = Rundown::import(SHOW, "s");
    let first = r.card_ids()[0];
    let last = *r.card_ids().last().unwrap();
    for edit in [
        Edit::MoveBefore(first),
        Edit::MoveAfter(last),
        Edit::MergeWithNext(last),
        Edit::Duplicate(CardId(999)),
    ] {
        let mut copy = r.clone();
        assert_eq!(copy.apply(edit), None, "{edit:?}");
        assert_eq!(copy, r, "{edit:?}");
    }
    let mut lone = Rundown::import("## only\n", "s");
    let only = lone.card_ids()[0];
    assert_eq!(lone.apply(Edit::Delete(only)), None);
    assert_eq!(lone.card_ids(), [only]);
}

#[test]
fn a_deleted_cards_id_is_never_given_to_a_new_card() {
    let mut r = Rundown::import(SHOW, "s");
    let two = id(&r, "Two");
    r.apply(Edit::Delete(two)).unwrap();
    let copy = r.apply(Edit::Duplicate(id(&r, "One"))).unwrap();
    assert_ne!(copy, two);
}

#[test]
fn merging_a_card_with_an_empty_cue_adds_no_stray_bullet() {
    let mut r = Rundown::import("## a\nbody a\n## \nbody b\n", "s");
    let a = r.card_ids()[0];
    r.apply(Edit::MergeWithNext(a)).unwrap();
    assert_eq!(r.sections[0].cards[0].body, "body a\nbody b");
}

#[test]
fn import_drops_a_section_with_no_cards() {
    let r = Rundown::import("# Empty\n# Real\n## card\n", "s");
    assert_eq!(layout(&r), sections(&[(Some("Real"), &["card"])]));
}

#[test]
fn a_document_keeps_its_deck_in_step_and_says_which_card_to_show() {
    let cues =
        |d: &Document| -> Vec<String> { d.deck().slides.iter().map(|s| s.cue.clone()).collect() };
    let mut d = Document::new("s".into(), Rundown::import("## a\n## b\n## c\n", "s"));
    assert_eq!(
        d.edit(2, "Delete Card", Edit::Delete),
        Some(1),
        "deleting the last card shows the one before"
    );
    assert_eq!(cues(&d), ["a", "b"]);
    assert_eq!(
        (d.undo_label(), d.redo_label()),
        (Some("Delete Card"), None)
    );
    assert_eq!(d.undo(1), Some(2), "undo shows the card that was current");
    assert_eq!(
        (d.undo_label(), d.redo_label()),
        (None, Some("Delete Card"))
    );
    assert_eq!(cues(&d), ["a", "b", "c"]);
    assert_eq!(d.redo(2), Some(1));
    assert_eq!(cues(&d), ["a", "b"]);
    assert_eq!(d.redo(1), None, "nothing left to redo");
    assert_eq!(
        d.edit(9, "Delete Card", Edit::Delete),
        None,
        "no card there"
    );
}
