use std::time::{Duration, Instant};

use keyprompter::deck::Deck;
use keyprompter::rundown::Rundown;
use keyprompter::show::{Command, Show};

fn parse(markdown: &str) -> Deck {
    Rundown::import(markdown, "").deck()
}

fn show(n: usize) -> Show {
    Show::new(parse(
        &(0..n).map(|i| format!("## s{i}\n")).collect::<String>(),
    ))
}

#[test]
fn first_advance_starts_the_timer_and_it_keeps_running() {
    let t0 = Instant::now();
    let mut s = show(3);
    assert_eq!(s.elapsed(t0), None);
    s.apply(Command::Next, t0);
    s.apply(Command::Next, t0 + Duration::from_secs(5));
    assert_eq!(
        s.elapsed(t0 + Duration::from_secs(10)),
        Some(Duration::from_secs(10))
    );
}

#[test]
fn prev_alone_does_not_start_the_timer() {
    let mut s = show(3);
    s.apply(Command::Prev, Instant::now());
    assert_eq!(s.elapsed(Instant::now()), None);
}

#[test]
fn cursor_clamps_at_both_ends() {
    let now = Instant::now();
    let mut s = show(3);
    s.apply(Command::Prev, now);
    assert_eq!(s.cursor(), 0);
    for _ in 0..10 {
        s.apply(Command::Next, now);
    }
    assert_eq!(s.cursor(), 2);
}

#[test]
fn first_and_last_jump_and_first_resets_the_timer() {
    let now = Instant::now();
    let mut s = show(4);
    s.apply(Command::Last, now);
    assert_eq!(s.cursor(), 3);
    s.apply(Command::Next, now);
    s.apply(Command::First, now);
    assert_eq!(s.cursor(), 0);
    assert_eq!(s.elapsed(now), None);
}

#[test]
fn reload_clamps_cursor_and_keeps_timer() {
    let now = Instant::now();
    let mut s = show(5);
    s.apply(Command::Next, now);
    s.apply(Command::Last, now);
    s.apply(Command::Reload(parse("## only\n## two\n")), now);
    assert_eq!(s.cursor(), 1);
    assert_eq!(s.elapsed(now), Some(Duration::ZERO));
    s.apply(Command::Reload(parse("")), now);
    assert_eq!(s.cursor(), 0);
    assert!(s.current().is_none());
}

#[test]
fn empty_deck_is_legal() {
    let mut s = show(0);
    s.apply(Command::Next, Instant::now());
    s.apply(Command::Last, Instant::now());
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
