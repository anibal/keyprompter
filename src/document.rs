use crate::{
    deck::Deck,
    rundown::{CardId, Edit, Rundown},
};

/// One undoable change: what it was called, and the rundown on the other
/// side of it with the card that was current there.
#[derive(Debug)]
struct Step {
    label: &'static str,
    rundown: Rundown,
    focus: Option<CardId>,
}

/// An open rundown, its library id, and its undo history. Every change goes
/// through here, so the deck on the strip always matches the rundown.
#[derive(Debug)]
pub struct Document {
    id: String,
    rundown: Rundown,
    deck: Deck,
    undo: Vec<Step>,
    redo: Vec<Step>,
}

impl Document {
    pub fn new(id: String, rundown: Rundown) -> Self {
        Self {
            deck: rundown.deck(),
            id,
            rundown,
            undo: Vec::new(),
            redo: Vec::new(),
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

    /// The name of the change Undo would revert.
    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo.last().map(|step| step.label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo.last().map(|step| step.label)
    }

    /// Applies `edit` to the card at `index`, remembering it as `label` for
    /// undo. On a change, returns the index of the card the edit focuses.
    pub fn edit(
        &mut self,
        index: usize,
        label: &'static str,
        edit: impl FnOnce(CardId) -> Edit,
    ) -> Option<usize> {
        let card = self.card(index)?;
        let mut next = self.rundown.clone();
        let focus = next.apply(edit(card))?;
        let rundown = std::mem::replace(&mut self.rundown, next);
        self.undo.push(Step {
            label,
            rundown,
            focus: Some(card),
        });
        self.redo.clear();
        Some(self.changed(Some(focus), index))
    }

    /// Steps back from the strip at `index`. Returns the index of the card
    /// that was current before the undone change.
    pub fn undo(&mut self, index: usize) -> Option<usize> {
        let step = self.undo.pop()?;
        Some(self.travel(step, index, true))
    }

    pub fn redo(&mut self, index: usize) -> Option<usize> {
        let step = self.redo.pop()?;
        Some(self.travel(step, index, false))
    }

    fn card(&self, index: usize) -> Option<CardId> {
        self.rundown.card_ids().get(index).copied()
    }

    fn travel(&mut self, step: Step, index: usize, back: bool) -> usize {
        let here = Step {
            label: step.label,
            rundown: self.rundown.clone(),
            focus: self.card(index),
        };
        if back { &mut self.redo } else { &mut self.undo }.push(here);
        // Ids handed out after the restored snapshot must stay retired.
        let next_id = self.rundown.next_id.max(step.rundown.next_id);
        self.rundown = step.rundown;
        self.rundown.next_id = next_id;
        self.changed(step.focus, index)
    }

    fn changed(&mut self, focus: Option<CardId>, index: usize) -> usize {
        self.deck = self.rundown.deck();
        let ids = self.rundown.card_ids();
        focus
            .and_then(|f| ids.iter().position(|&i| i == f))
            .unwrap_or_else(|| index.min(ids.len().saturating_sub(1)))
    }
}
