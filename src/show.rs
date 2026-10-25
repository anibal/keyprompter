use std::time::{Duration, Instant};

use crate::deck::{Deck, Slide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timer {
    Idle,
    Running { started: Instant },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Next,
    Prev,
    First,
    Last,
    Reload(Deck),
    Goto(usize),
}

#[derive(Debug, Clone)]
pub struct Show {
    deck: Deck,
    cursor: usize,
    timer: Timer,
}

impl Show {
    pub fn new(deck: Deck) -> Self {
        Self {
            deck,
            cursor: 0,
            timer: Timer::Idle,
        }
    }

    pub fn apply(&mut self, cmd: Command, now: Instant) {
        let last = self.deck.slides.len().saturating_sub(1);
        match cmd {
            Command::Next => {
                if self.timer == Timer::Idle {
                    self.timer = Timer::Running { started: now };
                }
                self.cursor = (self.cursor + 1).min(last);
            }
            Command::Prev => self.cursor = self.cursor.saturating_sub(1),
            Command::First => {
                self.cursor = 0;
                self.timer = Timer::Idle;
            }
            Command::Last => self.cursor = last,
            Command::Goto(index) => self.cursor = index.min(last),
            Command::Reload(deck) => {
                self.deck = deck;
                self.cursor = self.cursor.min(self.deck.slides.len().saturating_sub(1));
            }
        }
    }

    pub fn slides(&self) -> &[Slide] {
        &self.deck.slides
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn current(&self) -> Option<&Slide> {
        self.deck.slides.get(self.cursor)
    }

    pub fn elapsed(&self, now: Instant) -> Option<Duration> {
        match self.timer {
            Timer::Idle => None,
            Timer::Running { started } => Some(now.saturating_duration_since(started)),
        }
    }
}

/// `mm:ss`, or `--:--` before the first advance.
pub fn format_elapsed(elapsed: Option<Duration>) -> String {
    match elapsed {
        None => "--:--".into(),
        Some(d) => format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60),
    }
}
