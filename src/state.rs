use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Window rectangle in screen points, written as `x y w h`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Frame {
    pub fn parse(text: &str) -> Option<Self> {
        let mut it = text.split_whitespace().map(|v| v.parse::<f32>().ok());
        let (x, y, w, h) = (it.next()??, it.next()??, it.next()??, it.next()??);
        let sane = [x, y, w, h].iter().all(|v| v.is_finite()) && w > 0.0 && h > 0.0;
        (it.next().is_none() && sane).then_some(Self { x, y, w, h })
    }

    pub fn format(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.w, self.h)
    }
}

/// What survives a relaunch, stored as `key value` lines. Unknown keys and
/// malformed values are dropped, so a damaged file costs one setting, not all.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub frame: Option<Frame>,
    pub deck: Option<PathBuf>,
}

impl State {
    pub fn parse(text: &str) -> Self {
        let mut state = Self::default();
        for (key, value) in text.lines().filter_map(|l| l.split_once(' ')) {
            match key {
                "frame" => state.frame = Frame::parse(value),
                "deck" if !value.is_empty() => state.deck = Some(PathBuf::from(value)),
                _ => {}
            }
        }
        state
    }

    pub fn format(&self) -> String {
        let mut out = String::new();
        if let Some(frame) = self.frame {
            out += &format!("frame {}\n", frame.format());
        }
        if let Some(deck) = &self.deck {
            out += &format!("deck {}\n", deck.display());
        }
        out
    }
}

pub fn path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("KeyPrompter").join("state.txt"))
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
