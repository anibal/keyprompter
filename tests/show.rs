use std::time::{Duration, Instant};

use keyprompter::show::{Command, Show};

#[test]
fn first_advance_starts_the_timer_and_it_keeps_running() {
    let t0 = Instant::now();
    let (mut s, len) = (Show::default(), 3);
    assert_eq!(s.elapsed(t0), None);
    s.apply(Command::Next, len, t0);
    s.apply(Command::Next, len, t0 + Duration::from_secs(5));
    assert_eq!(
        s.elapsed(t0 + Duration::from_secs(10)),
        Some(Duration::from_secs(10))
    );
}

#[test]
fn prev_alone_does_not_start_the_timer() {
    let (mut s, len) = (Show::default(), 3);
    s.apply(Command::Prev, len, Instant::now());
    assert_eq!(s.elapsed(Instant::now()), None);
}

#[test]
fn cursor_clamps_at_both_ends() {
    let now = Instant::now();
    let (mut s, len) = (Show::default(), 3);
    s.apply(Command::Prev, len, now);
    assert_eq!(s.cursor(), 0);
    for _ in 0..10 {
        s.apply(Command::Next, len, now);
    }
    assert_eq!(s.cursor(), 2);
}

#[test]
fn first_and_last_jump_and_first_resets_the_timer() {
    let now = Instant::now();
    let (mut s, len) = (Show::default(), 4);
    s.apply(Command::Last, len, now);
    assert_eq!(s.cursor(), 3);
    s.apply(Command::Next, len, now);
    s.apply(Command::First, len, now);
    assert_eq!(s.cursor(), 0);
    assert_eq!(s.elapsed(now), None);
}

#[test]
fn empty_deck_is_legal() {
    let (mut s, len) = (Show::default(), 0);
    s.apply(Command::Next, len, Instant::now());
    s.apply(Command::Last, len, Instant::now());
    assert_eq!(s.cursor(), 0);
}

#[test]
fn elapsed_formats_as_minutes_and_seconds() {
    use keyprompter::show::format_elapsed;
    assert_eq!(format_elapsed(None), "--:--");
    assert_eq!(format_elapsed(Some(Duration::from_secs(65))), "01:05");
    assert_eq!(
        format_elapsed(Some(Duration::from_secs(61 * 60 + 9))),
        "61:09"
    );
}
