use crate::{
    deck::Deck,
    history::History,
    rundown::{CardId, Edit, Rundown},
};

/// The rundown with the card that was current, for undo.
type Snapshot = (Rundown, Option<CardId>);

/// An open rundown, its library id, and its undo history. Every change goes
/// through here, so the deck on the strip always matches the rundown.
#[derive(Debug)]
pub struct Document {
    id: String,
    rundown: Rundown,
    deck: Deck,
    history: History<Snapshot>,
}

impl Document {
    pub fn new(id: String, rundown: Rundown) -> Self {
        Self {
            deck: rundown.deck(),
            id,
            rundown,
            history: History::default(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn rundown(&self) -> &Rundown {
        &self.rundown
    }

    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    /// Applies `edit` to the card at `index`. On a change, returns the index of
    /// the card the edit focuses.
    pub fn edit(&mut self, index: usize, edit: fn(CardId) -> Edit) -> Option<usize> {
        let card = self.card(index)?;
        let mut next = self.rundown.clone();
        let focus = next.apply(edit(card))?;
        let previous = std::mem::replace(&mut self.rundown, next);
        self.history.record((previous, Some(card)));
        Some(self.changed(Some(focus), index))
    }

    /// Steps back from the strip at `index`. Returns the index of the card
    /// that was current before the undone change.
    pub fn undo(&mut self, index: usize) -> Option<usize> {
        self.travel(History::undo, index)
    }

    pub fn redo(&mut self, index: usize) -> Option<usize> {
        self.travel(History::redo, index)
    }

    fn card(&self, index: usize) -> Option<CardId> {
        self.rundown.card_ids().get(index).copied()
    }

    fn travel(
        &mut self,
        step: fn(&mut History<Snapshot>, Snapshot) -> Option<Snapshot>,
        index: usize,
    ) -> Option<usize> {
        let current = (self.rundown.clone(), self.card(index));
        let (restored, focus) = step(&mut self.history, current)?;
        // Ids handed out after the restored snapshot must stay retired.
        let next_id = self.rundown.next_id.max(restored.next_id);
        self.rundown = restored;
        self.rundown.next_id = next_id;
        Some(self.changed(focus, index))
    }

    fn changed(&mut self, focus: Option<CardId>, index: usize) -> usize {
        self.deck = self.rundown.deck();
        let ids = self.rundown.card_ids();
        focus
            .and_then(|f| ids.iter().position(|&i| i == f))
            .unwrap_or_else(|| index.min(ids.len().saturating_sub(1)))
    }
}
