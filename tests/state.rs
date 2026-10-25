use keyprompter::{
    snap::{Placement, Side, Stick},
    state::{Capture, State, load, save},
};

const SAVED: &str = "stick right\ntop 0\nheight 150\ndisplay Built-in Retina Display\nwidth left 900\nwidth right 1200\nrundown 1729800000000\ncapture shown\ntext -2\n";

#[test]
fn state_round_trips_through_the_file() {
    let dir = std::env::temp_dir().join(format!("kp-state-{}", std::process::id()));
    let path = dir.join("nested").join("state.txt");
    let state = State::parse(SAVED);
    let placement = state.placement.as_ref().unwrap();
    assert_eq!(placement.stick, Stick::Stuck(Side::Right));
    assert_eq!(placement.widths.get(Side::Left), Some(900.0));
    assert_eq!(placement.widths.get(Side::Center), None);
    assert_eq!(
        placement.display.as_deref(),
        Some("Built-in Retina Display")
    );
    assert_eq!(state.capture, Capture::Shown);
    assert_eq!(state.text, -2);
    save(&path, &state).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), SAVED);
    assert_eq!(load(&path), state);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_free_strip_saves_its_own_x_and_width() {
    let text = "stick free -120.5 1440\ntop 31\nheight 150\n";
    let placement = Placement::parse(&[
        ("stick", "free -120.5 1440"),
        ("top", "31"),
        ("height", "150"),
    ])
    .unwrap();
    assert_eq!(
        placement.stick,
        Stick::Free {
            x: -120.5,
            w: 1440.0
        }
    );
    assert_eq!(placement.format(), text);
    assert_eq!(State::parse(text).placement, Some(placement));
}

#[test]
fn a_legacy_frame_line_parses_as_a_free_strip_on_the_main_screen() {
    let state = State::parse("frame 300 31 1200 150\n");
    let placement = state.placement.unwrap();
    assert_eq!(
        placement.stick,
        Stick::Free {
            x: 300.0,
            w: 1200.0
        }
    );
    assert_eq!(
        (placement.top, placement.h, placement.display.as_deref()),
        (31.0, 150.0, None)
    );
    assert!(placement.format().starts_with("stick free 300 1200\n"));
    for bad in [
        "frame 1 2 3",
        "frame 1 2 3 x",
        "frame 1 2 3 4 5",
        "frame 0 0 0 150",
        "frame 0 0 NaN 150",
    ] {
        assert_eq!(State::parse(bad).placement, None, "{bad:?}");
    }
}

#[test]
fn a_missing_file_is_an_empty_state() {
    assert_eq!(load("/nonexistent/kp/state.txt".as_ref()), State::default());
}

#[test]
fn the_strip_starts_hidden_from_capture() {
    assert_eq!(State::parse("capture maybe\n").capture, Capture::Hidden);
    assert_eq!(State::default().capture, Capture::Hidden);
}

#[test]
fn a_bad_line_costs_only_its_own_setting() {
    let state = State::parse("stick sideways\nzoom 2\nrundown 42\n");
    assert_eq!(state.placement, None);
    assert_eq!(state.rundown.as_deref(), Some("42"));
    let state = State::parse(
        "stick left\ntop NaN\nheight 0\nwidth left abc\nwidth right 900\nwidth up 500\n",
    );
    let placement = state.placement.unwrap();
    assert_eq!((placement.top, placement.h), (0.0, 150.0));
    assert_eq!(placement.widths.get(Side::Left), None);
    assert_eq!(placement.widths.get(Side::Right), Some(900.0));
    assert_eq!(State::parse("stick free 1 2 3\n").placement, None);
    assert_eq!(State::parse("stick free 1\n").placement, None);
    assert_eq!(State::parse("top 31\nheight 150\n").placement, None);
}
