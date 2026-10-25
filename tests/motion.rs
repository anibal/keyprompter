use keyprompter::{
    motion::{Before, Motion},
    rundown::CardId,
};

const A: CardId = CardId(1);
const B: CardId = CardId(2);
const C: CardId = CardId(3);

fn before(ids: &[CardId]) -> Vec<Before<&'static str>> {
    ids.iter().map(|&id| Before { id, card: "old" }).collect()
}

fn run(motion: &mut Motion<&str>, ids: &[CardId], secs: f32, slide: bool) -> bool {
    let mut moving = true;
    for _ in 0..(secs * 60.0) as usize {
        moving = motion.step(ids, 1.0 / 60.0, slide);
    }
    moving
}

#[test]
fn a_moved_card_glides_from_its_old_place_to_its_new_one() {
    let mut m = Motion::default();
    let after = [B, A, C];
    m.change(before(&[A, B, C]), &after, &[], None);
    assert_eq!(m.look(A, 1).rank, 0.0, "starts where it was");
    assert_eq!(m.look(B, 0).rank, 1.0);
    m.step(&after, 0.05, true);
    let halfway = m.look(A, 1).rank;
    assert!(0.0 < halfway && halfway < 1.0, "{halfway}");
    assert!(!run(&mut m, &after, 2.0, true), "settles and forgets");
    assert_eq!(m.look(A, 1).rank, 1.0);
}

#[test]
fn a_deleted_card_fades_where_it_was() {
    let mut m = Motion::default();
    let after = [A, C];
    m.change(before(&[A, B, C]), &after, &[], None);
    assert_eq!(m.ghosts().len(), 1);
    assert_eq!((m.ghosts()[0].rank, m.ghosts()[0].into), (1.0, None));
    assert_eq!(m.look(C, 1).rank, 2.0, "the next card closes the gap");
    assert!(!run(&mut m, &after, 1.0, true));
    assert!(m.ghosts().is_empty());
}

#[test]
fn a_merged_card_slides_into_the_card_that_took_it() {
    let mut m = Motion::default();
    let after = [A, C];
    m.change(before(&[A, B, C]), &after, &[(A, 2)], Some(A));
    m.step(&after, 0.1, true);
    let ghost = &m.ghosts()[0];
    assert!(ghost.rank < 1.0 && ghost.fade < 1.0, "{ghost:?}");
    let a = m.look(A, 0);
    assert_eq!(
        (a.glow_from, a.glow > 0.0),
        (2, true),
        "lines after the old two glow"
    );
}

#[test]
fn a_new_card_fades_in_at_its_place() {
    let mut m = Motion::default();
    let after = [A, C, B];
    m.change(before(&[A, B]), &after, &[], None);
    assert_eq!((m.look(C, 1).rank, m.look(C, 1).appear), (1.0, 0.0));
    m.step(&after, 0.1, true);
    let c = m.look(C, 1).appear;
    assert!(0.0 < c && c < 1.0, "{c}");
}

#[test]
fn without_sliding_cards_jump_but_still_fade() {
    let mut m = Motion::default();
    let after = [B, A];
    m.change(before(&[A, B, C]), &after, &[], None);
    m.step(&after, 0.01, false);
    assert_eq!(m.look(A, 1).rank, 1.0);
    assert_eq!(m.ghosts()[0].rank, 2.0);
    assert!(m.ghosts()[0].fade < 1.0);
}

#[test]
fn a_card_brought_back_while_fading_out_carries_on_instead_of_showing_twice() {
    let mut m = Motion::default();
    m.change(before(&[A, B, C]), &[A, C], &[], None);
    m.step(&[A, C], 0.05, true);
    let fade = m.ghosts()[0].fade;
    m.change(before(&[A, C]), &[A, B, C], &[], None);
    assert!(m.ghosts().is_empty());
    assert_eq!(m.look(B, 1).appear, fade);
}

#[test]
fn without_sliding_a_merged_card_fades_where_it_was() {
    let mut m = Motion::default();
    m.change(before(&[A, B, C]), &[A, C], &[(A, 2)], Some(A));
    m.step(&[A, C], 0.05, false);
    assert_eq!(m.ghosts()[0].rank, 1.0);
}
