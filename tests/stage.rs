use keyprompter::stage::{ease, slot};

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
