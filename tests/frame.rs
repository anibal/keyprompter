use keyprompter::frame::{Frame, load, save};

#[test]
fn frame_round_trips_through_the_file() {
    let dir = std::env::temp_dir().join(format!("kp-frame-{}", std::process::id()));
    let path = dir.join("nested").join("window.txt");
    let frame = Frame {
        x: -120.5,
        y: 0.0,
        w: 1440.0,
        h: 150.0,
    };
    save(&path, frame).unwrap();
    assert_eq!(load(&path), Some(frame));
    std::fs::remove_dir_all(dir).unwrap();
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
