use keyprompter::snap::{Rect, settle};

const SCREEN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1920.0,
    h: 1080.0,
};
// AppKit coordinates: y grows upward, the menu bar takes the top 31 points.
const VISIBLE: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1920.0,
    h: 1049.0,
};

fn strip(x: f32, y: f32) -> Rect {
    Rect {
        x,
        y,
        w: 1200.0,
        h: 150.0,
    }
}

#[test]
fn a_strip_released_near_the_menu_bar_tucks_under_it() {
    assert_eq!(
        settle(strip(300.0, 890.0), SCREEN, VISIBLE, 24.0),
        Some((300.0, 899.0))
    );
}

#[test]
fn a_strip_released_near_the_very_top_sits_flush_with_it() {
    assert_eq!(
        settle(strip(300.0, 925.0), SCREEN, VISIBLE, 24.0),
        Some((300.0, 930.0))
    );
}

#[test]
fn both_axes_snap_into_a_corner() {
    assert_eq!(
        settle(strip(705.0, 10.0), SCREEN, VISIBLE, 24.0),
        Some((720.0, 0.0))
    );
}

#[test]
fn a_strip_away_from_every_edge_stays_put() {
    assert_eq!(settle(strip(300.0, 500.0), SCREEN, VISIBLE, 24.0), None);
}

#[test]
fn a_strip_already_on_an_edge_does_not_move_again() {
    assert_eq!(settle(strip(0.0, 899.0), SCREEN, VISIBLE, 24.0), None);
}
