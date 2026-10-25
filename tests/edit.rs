use keyprompter::{
    history::History,
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
        sections(&[
            (None, &["Open"]),
            (Some("A"), &["One", "Two"]),
            (Some("B"), &[])
        ])
    );
}

#[test]
fn moving_past_a_section_edge_joins_the_neighbouring_section() {
    let mut r = Rundown::import(SHOW, "s");
    let three = id(&r, "Three");
    r.apply(Edit::MoveEarlier(three)).unwrap();
    assert_eq!(
        layout(&r),
        sections(&[
            (None, &["Open"]),
            (Some("A"), &["One", "Two", "Three"]),
            (Some("B"), &[])
        ])
    );
    r.apply(Edit::MoveLater(three)).unwrap();
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
fn an_emptied_untitled_section_disappears_so_only_the_first_can_be_untitled() {
    let mut r = Rundown::import(SHOW, "s");
    r.apply(Edit::MoveLater(id(&r, "Open"))).unwrap();
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
        Edit::MoveEarlier(first),
        Edit::MoveLater(last),
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
fn undo_and_redo_walk_back_and_forth_and_a_new_step_clears_redo() {
    let mut history = History::default();
    history.record(1);
    history.record(2);
    assert_eq!(history.undo(3), Some(2));
    assert_eq!(history.undo(2), Some(1));
    assert_eq!(history.undo(1), None);
    assert_eq!(history.redo(1), Some(2));
    history.record(2);
    assert_eq!(history.redo(9), None);
}
