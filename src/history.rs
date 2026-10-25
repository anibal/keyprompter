/// Undo and redo over whole snapshots. Recording a new step clears redo.
#[derive(Debug)]
pub struct History<T> {
    past: Vec<T>,
    future: Vec<T>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
        }
    }
}

impl<T> History<T> {
    pub fn record(&mut self, before: T) {
        self.past.push(before);
        self.future.clear();
    }

    /// Hands back the state before the last step, keeping `current` for redo.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let previous = self.past.pop()?;
        self.future.push(current);
        Some(previous)
    }

    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.future.pop()?;
        self.past.push(current);
        Some(next)
    }

    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }
}
