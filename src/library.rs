use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::rundown::Rundown;

/// Rundowns saved as one JSON file each, named by id.
#[derive(Debug, Clone)]
pub struct Library {
    dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub title: String,
}

impl Library {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn default_dir() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("KeyPrompter").join("Rundowns"))
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    pub fn load(&self, id: &str) -> io::Result<Rundown> {
        serde_json::from_str(&fs::read_to_string(self.path(id))?).map_err(io::Error::other)
    }

    /// Writes beside the file and renames over it, so a crash mid-save
    /// leaves the previous version whole.
    pub fn save(&self, id: &str, rundown: &Rundown) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let temp = self.dir.join(format!(".{id}.json.tmp"));
        fs::write(
            &temp,
            serde_json::to_string_pretty(rundown).map_err(io::Error::other)?,
        )?;
        fs::rename(temp, self.path(id))
    }

    fn all(&self) -> Vec<(String, Rundown, SystemTime)> {
        let Ok(dir) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut all: Vec<_> = dir
            .flatten()
            .filter_map(|file| {
                let path = file.path();
                let id = path.file_stem()?.to_str()?.to_string();
                if path.extension()? != "json" || id.starts_with('.') {
                    return None;
                }
                let saved = file.metadata().and_then(|m| m.modified()).ok()?;
                Some((id.clone(), self.load(&id).ok()?, saved))
            })
            .collect();
        all.sort_by_key(|entry| std::cmp::Reverse(entry.2));
        all
    }

    /// Every readable rundown, most recently saved first.
    pub fn list(&self) -> Vec<Entry> {
        self.all()
            .into_iter()
            .map(|(id, rundown, _)| Entry {
                id,
                title: rundown.title,
            })
            .collect()
    }

    /// The library copy of a Markdown file. The first open imports it; later
    /// opens of the same file return that copy, edits included.
    pub fn open_markdown(&self, path: &Path) -> io::Result<(String, Rundown)> {
        if let Some((id, rundown, _)) = self
            .all()
            .into_iter()
            .find(|(_, r, _)| r.imported_from.as_deref() == Some(path))
        {
            return Ok((id, rundown));
        }
        let text = fs::read_to_string(path)?;
        let title = path
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        let mut rundown = Rundown::import(&text, title);
        rundown.imported_from = Some(path.to_path_buf());
        let id = self.new_id();
        self.save(&id, &rundown)?;
        Ok((id, rundown))
    }

    fn new_id(&self) -> String {
        let mut n = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        while self.path(&n.to_string()).exists() {
            n += 1;
        }
        n.to_string()
    }
}
