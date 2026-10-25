use keyprompter::snap::{
    Drag, Edge, Gesture, Lean, MIN_W, Placement, Point, Rect, Screen, Side, Stick, Widths,
    card_width, classify, edge_at, full_width, holding, hot_spot, lean, pushed, release,
};

// AppKit coordinates: y grows upward, the menu bar takes the top of each screen.
fn screen(name: &str, x: f32, w: f32, h: f32, bar: f32) -> Screen {
    Screen {
        name: name.into(),
        frame: Rect { x, y: 0.0, w, h },
        visible: Rect {
            x,
            y: 0.0,
            w,
            h: h - bar,
        },
    }
}

fn main_screen() -> Screen {
    screen("Studio Display", 0.0, 1920.0, 1080.0, 31.0)
}

fn laptop() -> Screen {
    screen("Built-in Retina Display", 1920.0, 1512.0, 982.0, 37.0)
}

const SCREEN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 1920.0,
    h: 1080.0,
};

fn one() -> Vec<Screen> {
    vec![main_screen()]
}

fn two() -> Vec<Screen> {
    vec![main_screen(), laptop()]
}

/// A strip `w` wide with its top `top` points below the screen's top edge.
fn strip(x: f32, y: f32, w: f32) -> Rect {
    Rect { x, y, w, h: 150.0 }
}

fn pt(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn stick_of(frame: Rect, screens: &[Screen]) -> Stick {
    classify(frame, screens, Widths::default()).unwrap().stick
}

#[test]
fn a_strip_is_stuck_where_an_edge_is_flush() {
    let stick = |x, w| stick_of(strip(x, 899.0, w), &one());
    assert_eq!(stick(0.0, 1200.0), Stick::Stuck(Side::Left));
    assert_eq!(stick(720.0, 1200.0), Stick::Stuck(Side::Right));
    assert_eq!(stick(360.0, 1200.0), Stick::Stuck(Side::Center));
    assert_eq!(stick(0.0, 1920.0), Stick::Stuck(Side::Center));
    assert_eq!(
        stick(300.0, 1200.0),
        Stick::Free {
            x: 300.0,
            w: 1200.0
        }
    );
    assert_eq!(stick(0.4, 1200.0), Stick::Stuck(Side::Left));
    assert_eq!(stick(1.0, 1200.0), Stick::Free { x: 1.0, w: 1200.0 });
}

#[test]
fn a_strip_on_the_second_display_is_classified_against_that_display() {
    let placement = classify(strip(1920.0, 800.0, 1000.0), &two(), Widths::default()).unwrap();
    assert_eq!(placement.stick, Stick::Stuck(Side::Left));
    assert_eq!(
        placement.display.as_deref(),
        Some("Built-in Retina Display")
    );
    assert_eq!(placement.top, 32.0);
    let free = classify(strip(2000.0, 800.0, 1000.0), &two(), Widths::default()).unwrap();
    assert_eq!(free.stick, Stick::Free { x: 80.0, w: 1000.0 });
}

#[test]
fn the_display_holding_most_of_the_strip_holds_it() {
    let screens = two();
    assert_eq!(
        holding(strip(1500.0, 800.0, 1200.0), &screens)
            .unwrap()
            .name,
        "Built-in Retina Display"
    );
    assert_eq!(
        holding(strip(1200.0, 800.0, 1200.0), &screens)
            .unwrap()
            .name,
        "Studio Display"
    );
    assert_eq!(
        holding(strip(9000.0, 800.0, 1200.0), &screens)
            .unwrap()
            .name,
        "Studio Display"
    );
    assert!(holding(strip(0.0, 800.0, 1200.0), &[]).is_none());
}

fn move_from(x: f32, y: f32) -> Drag {
    let frame = strip(x, y, 1200.0);
    Drag::start(
        Gesture::Move,
        frame,
        pt(x + 100.0, y + 50.0),
        Stick::Free { x, w: 1200.0 },
    )
}

#[test]
fn a_screen_edge_catches_the_strip_from_inside_and_holds_it_against_leaving() {
    let d = move_from(300.0, 500.0);
    let x = |px: f32| d.step(pt(px + 100.0, 550.0), false, &one()).x;
    assert_eq!(x(20.0), 0.0, "20 from flush, caught");
    assert_eq!(x(25.0), 25.0, "25 from flush, not yet");
    assert_eq!(x(-30.0), 0.0, "30 past the edge, held");
    assert_eq!(x(-48.0), 0.0, "48 past, still held");
    assert_eq!(x(-50.0), -50.0, "50 past, free");
    assert_eq!(x(700.0), 720.0, "20 from flush right, caught");
    assert_eq!(x(760.0), 720.0, "40 past the right edge, held");
    assert_eq!(x(770.0), 770.0, "50 past, free");
}

#[test]
fn the_center_line_catches_from_both_sides_within_catch_reach() {
    let d = move_from(300.0, 500.0);
    let x = |px: f32| d.step(pt(px + 100.0, 550.0), false, &one()).x;
    assert_eq!(x(340.0), 360.0);
    assert_eq!(x(380.0), 360.0);
    assert_eq!(x(390.0), 390.0);
}

#[test]
fn a_shared_display_edge_resists_from_both_sides() {
    let d = move_from(300.0, 500.0);
    let x = |px: f32| d.step(pt(px + 100.0, 550.0), false, &two()).x;
    assert_eq!(x(700.0), 720.0, "caught flush right on the main screen");
    assert_eq!(x(760.0), 720.0, "held on the main screen");
    assert_eq!(x(770.0), 770.0, "free, straddling");
    assert_eq!(
        x(1880.0),
        1920.0,
        "caught flush left on the laptop from 40 away"
    );
    assert_eq!(x(1940.0), 1920.0, "caught from inside the laptop");
    assert_eq!(x(1870.0), 1870.0, "50 short of the laptop, free");
}

#[test]
fn the_top_edge_and_the_menu_bar_catch_the_strip() {
    let d = move_from(300.0, 500.0);
    let y = |py: f32| d.step(pt(400.0, py + 50.0), false, &one()).y;
    assert_eq!(y(915.0), 930.0, "15 below the very top, caught");
    assert_eq!(y(970.0), 930.0, "40 over the top, held");
    assert_eq!(y(980.0), 980.0, "50 over the top, free");
    assert_eq!(y(880.0), 899.0, "19 below the menu bar, caught");
    assert_eq!(y(905.0), 899.0, "6 above the menu bar, caught");
    assert_eq!(y(10.0), 10.0, "no magnet at the bottom");
}

fn resize_left_stuck() -> Drag {
    Drag::start(
        Gesture::Resize(Edge::Right),
        strip(0.0, 899.0, 1200.0),
        pt(1200.0, 950.0),
        Stick::Stuck(Side::Left),
    )
}

#[test]
fn a_resize_never_goes_under_the_floor_or_past_the_screen() {
    let d = resize_left_stuck();
    assert_eq!(d.step(pt(500.0, 950.0), false, &one()).w, MIN_W);
    assert_eq!(
        d.step(pt(2500.0, 950.0), false, &one()),
        strip(0.0, 899.0, 1920.0)
    );
    let narrow = [screen("Tiny", 0.0, 500.0, 400.0, 25.0)];
    let d = Drag::start(
        Gesture::Resize(Edge::Right),
        strip(0.0, 200.0, 500.0),
        pt(500.0, 250.0),
        Stick::Stuck(Side::Center),
    );
    assert_eq!(d.step(pt(100.0, 250.0), false, &narrow).w, 500.0);
}

#[test]
fn option_mirrors_the_other_edge_and_unsticks_the_strip() {
    let d = resize_left_stuck();
    let mirrored = d.step(pt(1000.0, 950.0), true, &one());
    assert_eq!(mirrored, strip(200.0, 899.0, 800.0));
    assert_eq!(
        stick_of(mirrored, &one()),
        Stick::Free { x: 200.0, w: 800.0 }
    );
    assert_eq!(
        d.step(pt(1000.0, 950.0), false, &one()),
        strip(0.0, 899.0, 1000.0)
    );
}

#[test]
fn a_centered_strip_resizes_from_both_edges_at_once() {
    let d = Drag::start(
        Gesture::Resize(Edge::Left),
        strip(360.0, 899.0, 1200.0),
        pt(360.0, 950.0),
        Stick::Stuck(Side::Center),
    );
    assert_eq!(
        d.step(pt(460.0, 950.0), false, &one()),
        strip(460.0, 899.0, 1000.0)
    );
    assert_eq!(
        d.step(pt(-100.0, 950.0), false, &one()),
        strip(0.0, 899.0, 1920.0)
    );
    assert_eq!(d.step(pt(800.0, 950.0), false, &one()).w, MIN_W);
}

#[test]
fn a_resized_edge_is_caught_by_the_screen_edge_and_the_center_line() {
    let d = resize_left_stuck();
    assert_eq!(d.step(pt(1900.0, 950.0), false, &one()).w, 1920.0);
    assert_eq!(d.step(pt(950.0, 950.0), false, &one()).w, 960.0);
    assert_eq!(d.step(pt(930.0, 950.0), false, &one()).w, 930.0);
    let d = Drag::start(
        Gesture::Resize(Edge::Left),
        strip(720.0, 899.0, 1200.0),
        pt(720.0, 950.0),
        Stick::Stuck(Side::Right),
    );
    assert_eq!(
        d.step(pt(15.0, 950.0), false, &one()),
        strip(0.0, 899.0, 1920.0)
    );
}

#[test]
fn the_stuck_edge_cannot_be_grabbed() {
    assert_eq!(edge_at(3.0, 1200.0, Stick::Stuck(Side::Left)), None);
    assert_eq!(
        edge_at(1197.0, 1200.0, Stick::Stuck(Side::Left)),
        Some(Edge::Right)
    );
    assert_eq!(edge_at(1197.0, 1200.0, Stick::Stuck(Side::Right)), None);
    assert_eq!(
        edge_at(3.0, 1200.0, Stick::Stuck(Side::Right)),
        Some(Edge::Left)
    );
    assert_eq!(
        edge_at(3.0, 1920.0, Stick::Stuck(Side::Center)),
        Some(Edge::Left)
    );
    assert_eq!(
        edge_at(1917.0, 1920.0, Stick::Stuck(Side::Center)),
        Some(Edge::Right)
    );
    let free = Stick::Free {
        x: 300.0,
        w: 1200.0,
    };
    assert_eq!(edge_at(7.9, 1200.0, free), Some(Edge::Left));
    assert_eq!(edge_at(8.0, 1200.0, free), None);
    assert_eq!(edge_at(600.0, 1200.0, free), None);
}

#[test]
fn the_cursor_leans_one_way_at_the_floor_and_at_the_screen_edge() {
    let s = main_screen();
    assert_eq!(
        lean(Edge::Right, strip(0.0, 899.0, 1200.0), &s, false),
        Lean::Both
    );
    assert_eq!(
        lean(Edge::Right, strip(0.0, 899.0, MIN_W), &s, false),
        Lean::Outward
    );
    assert_eq!(
        lean(Edge::Right, strip(0.0, 899.0, 1920.0), &s, false),
        Lean::Inward
    );
    assert_eq!(
        lean(Edge::Left, strip(720.0, 899.0, 1200.0), &s, false),
        Lean::Both
    );
    assert_eq!(
        lean(Edge::Left, strip(0.0, 899.0, 1200.0), &s, false),
        Lean::Inward
    );
    assert_eq!(
        lean(Edge::Left, strip(0.0, 899.0, 1920.0), &s, true),
        Lean::Inward
    );
    assert_eq!(
        lean(Edge::Left, strip(360.0, 899.0, 1200.0), &s, true),
        Lean::Both
    );
}

#[test]
fn a_double_click_pushes_the_edge_out_and_option_makes_it_full_width() {
    let s = main_screen();
    assert_eq!(
        pushed(strip(300.0, 899.0, 1200.0), Edge::Right, &s),
        strip(300.0, 899.0, 1620.0)
    );
    assert_eq!(
        pushed(strip(300.0, 899.0, 1200.0), Edge::Left, &s),
        strip(0.0, 899.0, 1500.0)
    );
    assert_eq!(
        full_width(strip(300.0, 899.0, 1200.0), &s),
        strip(0.0, 899.0, 1920.0)
    );
}

#[test]
fn a_released_strip_lands_on_the_display_holding_most_of_it() {
    let screens = two();
    assert_eq!(
        release(strip(1500.0, 800.0, 1200.0), &screens),
        strip(1920.0, 800.0, 1200.0)
    );
    assert_eq!(
        release(strip(1200.0, 800.0, 1200.0), &screens),
        strip(720.0, 800.0, 1200.0)
    );
    assert_eq!(
        release(strip(1900.0, 800.0, 1600.0), &screens),
        strip(1920.0, 800.0, 1512.0)
    );
    assert_eq!(
        release(strip(-100.0, 800.0, 1200.0), &screens),
        strip(-100.0, 800.0, 1200.0)
    );
    assert_eq!(
        release(strip(9000.0, 800.0, 1200.0), &screens),
        strip(720.0, 800.0, 1200.0)
    );
}

#[test]
fn each_side_remembers_the_width_the_strip_last_had_there() {
    let screens = one();
    let left = classify(strip(0.0, 899.0, 900.0), &screens, Widths::default()).unwrap();
    let free = classify(strip(300.0, 899.0, 1200.0), &screens, left.widths).unwrap();
    assert_eq!(free.widths.get(Side::Left), Some(900.0));
    assert_eq!(free.widths.get(Side::Right), None);
    let s = main_screen();
    assert_eq!(
        free.stuck(Side::Left, strip(300.0, 899.0, 1200.0), &s),
        strip(0.0, 899.0, 900.0)
    );
    assert_eq!(
        free.stuck(Side::Right, strip(300.0, 899.0, 1200.0), &s),
        strip(720.0, 899.0, 1200.0)
    );
    assert_eq!(
        free.stuck(Side::Center, strip(300.0, 899.0, 1200.0), &s),
        strip(360.0, 899.0, 1200.0)
    );
    let center = classify(strip(0.0, 899.0, 1920.0), &screens, free.widths).unwrap();
    assert_eq!(center.widths.get(Side::Center), Some(1920.0));
    assert_eq!(center.widths.get(Side::Left), Some(900.0));
}

#[test]
fn a_saved_placement_rebuilds_its_frame_on_the_screens_connected_now() {
    let right = classify(strip(720.0, 899.0, 1200.0), &one(), Widths::default()).unwrap();
    let smaller = [screen("Studio Display", 0.0, 1440.0, 900.0, 31.0)];
    assert_eq!(right.frame_on(&smaller), Some(strip(240.0, 719.0, 1200.0)));
    let gone = Placement {
        display: Some("Projector".into()),
        ..right.clone()
    };
    assert_eq!(gone.frame_on(&two()), Some(strip(720.0, 899.0, 1200.0)));
    let free = classify(strip(1500.0, 899.0, 1200.0), &one(), Widths::default()).unwrap();
    assert_eq!(free.frame_on(&smaller), Some(strip(240.0, 719.0, 1200.0)));
    let fresh = Placement {
        stick: Stick::Stuck(Side::Left),
        top: 31.0,
        h: 150.0,
        display: None,
        widths: Widths::default(),
    };
    assert_eq!(fresh.frame_on(&one()), Some(strip(0.0, 899.0, 1200.0)));
    assert_eq!(fresh.frame_on(&[]), None);
}

#[test]
fn the_hot_spot_is_the_screen_center_as_far_as_the_strip_reaches() {
    assert_eq!(
        hot_spot(strip(360.0, 899.0, 1200.0), SCREEN, 200.0, 240.0),
        400.0
    );
    assert_eq!(
        hot_spot(strip(1100.0, 899.0, 800.0), SCREEN, 200.0, 240.0),
        120.0
    );
    assert_eq!(
        hot_spot(strip(0.0, 899.0, 700.0), SCREEN, 200.0, 210.0),
        395.0
    );
    assert_eq!(
        hot_spot(strip(1220.0, 899.0, 700.0), SCREEN, 200.0, 210.0),
        105.0
    );
}

#[test]
fn cards_take_a_third_of_the_strip_up_to_a_cap() {
    assert_eq!(card_width(1000.0), 300.0);
    assert_eq!(card_width(1920.0), 420.0);
}
