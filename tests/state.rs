use keyprompter::state::{Capture, Frame, State, load, save};

#[test]
fn state_round_trips_through_the_file() {
    let dir = std::env::temp_dir().join(format!("kp-state-{}", std::process::id()));
    let path = dir.join("nested").join("state.txt");
    let state = State {
        frame: Some(Frame {
            x: -120.5,
            y: 0.0,
            w: 1440.0,
            h: 150.0,
        }),
        rundown: Some("1729800000000".into()),
        capture: Capture::Shown,
        text: -2,
    };
    save(&path, &state).unwrap();
    assert_eq!(load(&path), state);
    std::fs::remove_dir_all(dir).unwrap();
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
    let state = State::parse("frame 0 0 NaN 150\nzoom 2\nrundown 42\n");
    assert_eq!(state.frame, None);
    assert_eq!(state.rundown.as_deref(), Some("42"));
}

#[test]
fn unparseable_frames_are_rejected() {
    for bad in [
        "",
        "1 2 3",
        "1 2 3 x",
        "1 2 3 4 5",
        "0 0 0 150",
        "0 0 NaN 150",
        "0 0 -5 150",
    ] {
        assert_eq!(Frame::parse(bad), None, "{bad:?}");
    }
}
