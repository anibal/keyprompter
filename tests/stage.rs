use keyprompter::stage::{Stage, ease, label_span, slot};

#[test]
fn hot_spot_is_centered_at_full_size() {
    let s = slot(0.0);
    assert_eq!((s.x, s.scale, s.opacity), (0.0, 1.0, 1.0));
}

#[test]
fn upcoming_go_right_smaller_and_past_go_left_smaller_still() {
    let next = slot(1.0);
    let prev = slot(-1.0);
    assert!(next.x > 0.5 && prev.x < -0.5);
    assert!(prev.scale < next.scale && next.scale < 1.0);
    assert!(prev.opacity < next.opacity && next.opacity < 1.0);
}

#[test]
fn two_or_three_upcoming_are_visible_and_far_ones_are_not() {
    assert!(slot(2.0).opacity > 0.5);
    assert!(slot(3.0).opacity > 0.0);
    assert_eq!(slot(3.6).opacity, 0.0);
    assert_eq!(slot(-3.6).opacity, 0.0);
}

#[test]
fn slot_is_continuous_across_d() {
    let mut d = -5.0_f32;
    let mut prev = slot(d);
    while d < 5.0 {
        d += 0.01;
        let s = slot(d);
        assert!((s.x - prev.x).abs() < 0.02, "x jumps at d={d}");
        assert!((s.scale - prev.scale).abs() < 0.01, "scale jumps at d={d}");
        assert!(
            (s.opacity - prev.opacity).abs() < 0.02,
            "opacity jumps at d={d}"
        );
        prev = s;
    }
}

#[test]
fn ease_converges_and_snaps_exactly() {
    let mut pos = 0.0;
    let mut frames = 0;
    while pos != 3.0 {
        pos = ease(pos, 3.0, 1.0 / 60.0);
        frames += 1;
        assert!(frames < 600, "never settled");
    }
}

#[test]
fn ease_moves_monotonically_and_retargets_from_current_pos() {
    let mid = ease(0.0, 1.0, 0.05);
    assert!(mid > 0.0 && mid < 1.0);
    let back = ease(mid, 0.0, 0.05);
    assert!(back < mid && back >= 0.0);
}

#[test]
fn ease_result_does_not_depend_on_frame_rate() {
    let coarse = ease(0.0, 1.0, 0.1);
    let fine = (0..10).fold(0.0, |p, _| ease(p, 1.0, 0.01));
    assert!((coarse - fine).abs() < 1e-3);
}

const STAGE: Stage = Stage {
    hot_x: 500.0,
    card_w: 300.0,
    card_h: 130.0,
    win_h: 150.0,
};

#[test]
fn the_middle_card_is_centered_on_the_hot_spot_at_full_size() {
    let p = STAGE.place(0.0, 1.0);
    assert_eq!((p.w, p.h, p.size), (300.0, 130.0, 1.0));
    assert_eq!(p.left + p.w / 2.0, 500.0);
}

#[test]
fn a_next_card_sits_right_of_the_middle_one_and_smaller() {
    let middle = STAGE.place(0.0, 1.0);
    let next = STAGE.place(1.0, 1.0);
    assert!(next.left > middle.left + middle.w / 2.0);
    assert!(next.w < middle.w && next.h < middle.h);
}

#[test]
fn side_cards_on_both_sides_hang_from_one_top() {
    let hang = STAGE.place(1.0, 1.0).top;
    assert_eq!(hang, STAGE.hang_top());
    for d in [-3.0, -1.0, 1.0, 2.5] {
        assert_eq!(STAGE.place(d, 1.0).top, hang);
    }
    assert!((hang - (10.0 + 130.0 * 0.2)).abs() < 1e-4);
}

#[test]
fn an_upcoming_card_ends_where_the_middle_card_ends() {
    let (mid, next) = (STAGE.place(0.0, 1.0), STAGE.place(1.0, 1.0));
    assert!((next.top + next.h - (mid.top + mid.h)).abs() < 1e-4);
}

#[test]
fn shrinking_cards_keep_their_top() {
    assert_eq!(STAGE.place(1.0, 0.5).top, STAGE.place(1.0, 1.0).top);
}

#[test]
fn top_is_continuous_across_d() {
    let mut d = -4.0_f32;
    let mut prev = STAGE.place(d, 1.0).top;
    while d < 4.0 {
        d += 0.01;
        let top = STAGE.place(d, 1.0).top;
        assert!((top - prev).abs() < 0.5, "top jumps at d={d}");
        prev = top;
    }
}

#[test]
fn shrink_scales_a_card_around_its_center() {
    let full = STAGE.place(1.0, 1.0);
    let small = STAGE.place(1.0, 0.5);
    assert!((small.w - full.w / 2.0).abs() < 1e-4);
    assert!((small.left + small.w / 2.0 - (full.left + full.w / 2.0)).abs() < 1e-4);
}

const VISIBLE: (f32, f32) = (0.0, 800.0);

#[test]
fn a_label_starts_at_the_first_pieces_left() {
    assert_eq!(
        label_span(&[(100.0, 300.0), (500.0, 700.0)], VISIBLE, 40.0),
        Some((100.0, 300.0))
    );
}

#[test]
fn a_label_is_pinned_to_the_stage_edge_when_the_piece_starts_off_screen() {
    assert_eq!(
        label_span(&[(-400.0, 250.0), (500.0, 700.0)], VISIBLE, 40.0),
        Some((0.0, 250.0))
    );
}

#[test]
fn a_label_skips_a_narrow_left_piece_for_the_right_one() {
    assert_eq!(
        label_span(&[(-400.0, 30.0), (500.0, 700.0)], VISIBLE, 40.0),
        Some((500.0, 700.0))
    );
    assert_eq!(
        label_span(&[(100.0, 120.0), (500.0, 700.0)], VISIBLE, 40.0),
        Some((500.0, 700.0))
    );
}

#[test]
fn a_label_is_dropped_when_nothing_is_wide_enough() {
    assert_eq!(
        label_span(&[(-400.0, 30.0), (770.0, 900.0)], VISIBLE, 40.0),
        None
    );
    assert_eq!(label_span(&[], VISIBLE, 40.0), None);
}

fn right_edge(d: f32) -> f32 {
    let p = STAGE.place(d, 1.0);
    p.left + p.w
}

#[test]
fn a_run_around_the_middle_has_a_piece_each_side_between_side_card_edges() {
    let pieces = STAGE.header_pieces(-2.0, 2.0);
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0], (STAGE.place(-2.0, 1.0).left, right_edge(-1.0)));
    assert_eq!(pieces[1], (STAGE.place(1.0, 1.0).left, right_edge(2.0)));
}

#[test]
fn a_run_of_only_the_middle_card_has_no_pieces() {
    assert!(STAGE.header_pieces(0.0, 0.0).is_empty());
}

#[test]
fn a_run_on_one_side_has_one_piece_across_its_cards() {
    assert_eq!(
        STAGE.header_pieces(1.0, 3.0),
        vec![(STAGE.place(1.0, 1.0).left, right_edge(3.0))]
    );
}

#[test]
fn pieces_vary_continuously_as_the_strip_slides() {
    let mut pos = 0.0_f32;
    let mut prev = STAGE.header_pieces(-2.0 - pos, 2.0 - pos);
    while pos < 3.0 {
        pos += 0.01;
        let now = STAGE.header_pieces(-2.0 - pos, 2.0 - pos);
        if now.len() == prev.len() {
            for (a, b) in now.iter().zip(&prev) {
                assert!(
                    (a.0 - b.0).abs() < 20.0 && (a.1 - b.1).abs() < 20.0,
                    "jump at pos={pos}"
                );
            }
        } else {
            let widths: Vec<_> = now.iter().chain(&prev).map(|(a, b)| b - a).collect();
            assert!(widths.iter().any(|w| *w < 20.0), "piece pops at pos={pos}");
        }
        prev = now;
    }
}
