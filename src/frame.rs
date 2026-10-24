use std::{fs, io, path::PathBuf};

/// Window rectangle in screen points, persisted as `x y w h`.
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

pub fn state_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("KeyPrompter").join("window.txt"))
}

pub fn load(path: &std::path::Path) -> Option<Frame> {
    Frame::parse(&fs::read_to_string(path).ok()?)
}

pub fn save(path: &std::path::Path, frame: Frame) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, frame.format())
}
