use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Timer {
    #[default]
    Idle,
    Running {
        started: Instant,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Next,
    Prev,
    First,
    Last,
    Goto(usize),
}

/// Where the strip is in a deck of cards, and since when it has been running.
#[derive(Debug, Clone, Default)]
pub struct Show {
    cursor: usize,
    timer: Timer,
}

impl Show {
    /// Moves within a deck of `len` cards.
    pub fn apply(&mut self, cmd: Command, len: usize, now: Instant) {
        let last = len.saturating_sub(1);
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
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
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
