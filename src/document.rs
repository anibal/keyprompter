use crate::{
    deck::Deck,
    rundown::{CardId, Edit, Rundown, Side},
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

    /// Applies `edit` to the card at `target` while the card at `current` is
    /// in the hot spot, remembering it as `label` for undo. `edit` learns on
    /// which side of the card the hot spot is. On a change, returns the index
    /// to show. That is the hot-spot card wherever it went, unless the edit
    /// was on it or removed it, in which case it is the card the edit focuses.
    pub fn edit(
        &mut self,
        current: usize,
        target: usize,
        label: &'static str,
        edit: impl FnOnce(CardId, Side) -> Edit,
    ) -> Option<usize> {
        let card = self.card(target)?;
        let hot = self.card(current);
        let side = if target > current {
            Side::Before
        } else {
            Side::After
        };
        let mut next = self.rundown.clone();
        let focus = next.apply(edit(card, side))?;
        let rundown = std::mem::replace(&mut self.rundown, next);
        self.undo.push(Step {
            label,
            rundown,
            focus: hot,
        });
        self.redo.clear();
        let stays = hot.filter(|&h| h != card && self.rundown.card_ids().contains(&h));
        Some(self.changed(stays.or(Some(focus)), current))
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
