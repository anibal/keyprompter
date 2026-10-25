use std::{fs, io, path::Path};

use crate::snap::{DEFAULT_H, Placement, Side, Stick, Widths};

const SIDES: [(Side, &str); 3] = [
    (Side::Left, "left"),
    (Side::Center, "center"),
    (Side::Right, "right"),
];

fn side(name: &str) -> Option<Side> {
    SIDES.iter().find(|(_, n)| *n == name).map(|(s, _)| *s)
}

fn finite(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|n| n.is_finite())
}

impl Placement {
    /// Lines in, placement out. A `stick` line is required; `top` and
    /// `height` default, and a `width <side>` line with a bad number drops
    /// only that width. A legacy `frame x y w h` line, which scripts/try.sh
    /// still writes, parses as `Free { x, w }` with `top: y` on the main
    /// screen, so an old file is rewritten in the new keys on the next save.
    pub fn parse(lines: &[(&str, &str)]) -> Option<Self> {
        let last = |key: &str| lines.iter().rev().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let mut top = last("top").and_then(finite);
        let mut h = last("height").and_then(finite).filter(|h| *h > 0.0);
        let stick = match last("stick") {
            Some(value) => {
                let mut words = value.split_whitespace();
                let stick = match words.next()? {
                    "free" => Stick::Free {
                        x: finite(words.next()?)?,
                        w: finite(words.next()?)?,
                    },
                    word => Stick::Stuck(side(word)?),
                };
                if words.next().is_some() {
                    return None;
                }
                stick
            }
            None => {
                let mut numbers = last("frame")?.split_whitespace().map(finite);
                let (x, y, w, fh) = (
                    numbers.next()??,
                    numbers.next()??,
                    numbers.next()??,
                    numbers.next()??,
                );
                if numbers.next().is_some() || w <= 0.0 || fh <= 0.0 {
                    return None;
                }
                top = Some(y);
                h = Some(fh);
                Stick::Free { x, w }
            }
        };
        let widths = lines
            .iter()
            .filter(|(k, _)| *k == "width")
            .filter_map(|(_, v)| {
                let (name, w) = v.split_once(' ')?;
                Some((side(name)?, finite(w)?))
            })
            .fold(Widths::default(), |widths, (side, w)| widths.with(side, w));
        Some(Self {
            stick,
            top: top.unwrap_or(0.0),
            h: h.unwrap_or(DEFAULT_H),
            display: last("display").map(str::to_owned),
            widths,
        })
    }

    pub fn format(&self) -> String {
        let stick = match self.stick {
            Stick::Stuck(side) => SIDES[side as usize].1.to_string(),
            Stick::Free { x, w } => format!("free {x} {w}"),
        };
        let mut out = format!("stick {stick}\ntop {}\nheight {}\n", self.top, self.h);
        if let Some(display) = &self.display {
            out += &format!("display {display}\n");
        }
        for (side, name) in SIDES {
            if let Some(w) = self.widths.get(side) {
                out += &format!("width {name} {w}\n");
            }
        }
        out
    }
}

/// What survives a relaunch, stored as `key value` lines. Unknown keys and
/// malformed values are dropped, so a damaged file costs one setting, not all.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub placement: Option<Placement>,
    /// Library id of the open rundown.
    pub rundown: Option<String>,
    pub capture: Capture,
    /// Text-size steps away from the size the strip height gives.
    pub text: i32,
}

/// Whether screen captures and recordings include the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Capture {
    #[default]
    Hidden,
    Shown,
}

impl State {
    pub fn parse(text: &str) -> Self {
        let lines: Vec<_> = text.lines().filter_map(|l| l.split_once(' ')).collect();
        let mut state = Self {
            placement: Placement::parse(&lines),
            ..Self::default()
        };
        for (key, value) in lines {
            match key {
                "rundown" if !value.is_empty() => state.rundown = Some(value.to_string()),
                "capture" if value == "shown" => state.capture = Capture::Shown,
                "text" => state.text = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        state
    }

    pub fn format(&self) -> String {
        let mut out = String::new();
        if let Some(placement) = &self.placement {
            out += &placement.format();
        }
        if let Some(id) = &self.rundown {
            out += &format!("rundown {id}\n");
        }
        if self.capture == Capture::Shown {
            out += "capture shown\n";
        }
        if self.text != 0 {
            out += &format!("text {}\n", self.text);
        }
        out
    }
}

pub fn load(path: &Path) -> State {
    fs::read_to_string(path).map_or_else(|_| State::default(), |t| State::parse(&t))
}

pub fn save(path: &Path, state: &State) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, state.format())
}
