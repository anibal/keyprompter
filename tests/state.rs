use std::path::PathBuf;

use keyprompter::state::{Frame, State, load, save};

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
        deck: Some(PathBuf::from("/Users/me/My Shows/rundown.md")),
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
fn a_bad_line_costs_only_its_own_setting() {
    let state = State::parse("frame 0 0 NaN 150\nzoom 2\ndeck /tmp/a.md\n");
    assert_eq!(state.frame, None);
    assert_eq!(state.deck, Some(PathBuf::from("/tmp/a.md")));
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
