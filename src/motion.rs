use std::collections::HashMap;

use crate::{rundown::CardId, stage::ease};

const APPEAR_SECS: f32 = 0.25;
/// Shorter than the slide, so a removed card is mostly gone before its
/// neighbours pass over it.
const VANISH_SECS: f32 = 0.18;
const GLOW_SECS: f32 = 0.9;

/// How one card is drawn while the strip settles after an edit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// Where the card sits, in slide indices.
    pub rank: f32,
    /// 0 when the card has just appeared, 1 once it is fully in.
    pub appear: f32,
    /// Body lines from this one on are new, and glow at `glow` strength.
    pub glow_from: usize,
    pub glow: f32,
}

/// A card an edit removed, drawn until it has faded out. It shrinks in place,
/// or slides into the card it was merged into.
#[derive(Debug, Clone)]
pub struct Ghost<T> {
    pub id: CardId,
    pub card: T,
    pub rank: f32,
    pub into: Option<CardId>,
    pub fade: f32,
}

/// What the strip animates after an edit, keyed by card id so a card glides
/// from where it was to where it now is. Empty at rest.
#[derive(Debug, Clone)]
pub struct Motion<T> {
    looks: HashMap<CardId, Look>,
    ghosts: Vec<Ghost<T>>,
}

impl<T> Default for Motion<T> {
    fn default() -> Self {
        Self {
            looks: HashMap::new(),
            ghosts: Vec::new(),
        }
    }
}

/// One card before an edit: its id, and what to draw for it if it goes away.
pub struct Before<T> {
    pub id: CardId,
    pub card: T,
}

impl<T> Motion<T> {
    pub fn clear(&mut self) {
        self.looks.clear();
        self.ghosts.clear();
    }

    /// Starts the animation from the deck `before` an edit to the one `after`
    /// it. `changed` names the cards the edit changed in place, each with the
    /// first body line that should glow, and `into` the card that absorbed
    /// any removed ones.
    pub fn change(
        &mut self,
        before: Vec<Before<T>>,
        after: &[CardId],
        changed: &[(CardId, usize)],
        into: Option<CardId>,
    ) {
        for (index, old) in before.into_iter().enumerate() {
            let look = self.look(old.id, index);
            if after.contains(&old.id) {
                let entry = self.looks.entry(old.id).or_insert(look);
                if let Some(&(_, from)) = changed.iter().find(|(id, _)| *id == old.id) {
                    entry.glow_from = from;
                    entry.glow = 1.0;
                }
            } else {
                self.looks.remove(&old.id);
                self.ghosts.push(Ghost {
                    id: old.id,
                    card: old.card,
                    rank: look.rank,
                    into,
                    fade: look.appear,
                });
            }
        }
        for (index, &id) in after.iter().enumerate() {
            if self.looks.contains_key(&id) {
                continue;
            }
            // A card brought back while it was still fading out carries on
            // from there instead of showing twice.
            let (rank, appear) = match self.ghosts.iter().position(|g| g.id == id) {
                Some(i) => {
                    let ghost = self.ghosts.remove(i);
                    (ghost.rank, ghost.fade)
                }
                None => (index as f32, 0.0),
            };
            self.looks.insert(
                id,
                Look {
                    rank,
                    appear,
                    glow_from: 0,
                    glow: 0.0,
                },
            );
        }
    }

    /// How the card at `index` of the current deck is drawn.
    pub fn look(&self, id: CardId, index: usize) -> Look {
        self.looks.get(&id).copied().unwrap_or(Look {
            rank: index as f32,
            appear: 1.0,
            glow_from: 0,
            glow: 0.0,
        })
    }

    pub fn ghosts(&self) -> &[Ghost<T>] {
        &self.ghosts
    }

    /// Advances by `dt` seconds toward the deck `ids`. Without `slide`, as
    /// under Reduce Motion, cards jump to their places and only fade.
    /// Returns whether anything is still moving.
    pub fn step(&mut self, ids: &[CardId], dt: f32, slide: bool) -> bool {
        for (index, id) in ids.iter().enumerate() {
            if let Some(look) = self.looks.get_mut(id) {
                let target = index as f32;
                look.rank = if slide {
                    ease(look.rank, target, dt)
                } else {
                    target
                };
                look.appear = (look.appear + dt / APPEAR_SECS).min(1.0);
                look.glow = (look.glow - dt / GLOW_SECS).max(0.0);
                if look.rank == target && look.appear == 1.0 && look.glow == 0.0 {
                    self.looks.remove(id);
                }
            }
        }
        for ghost in &mut self.ghosts {
            ghost.fade = (ghost.fade - dt / VANISH_SECS).max(0.0);
            // Without sliding, a merged card fades where it was rather than
            // over the text of the card that took it.
            if slide
                && let Some(target) = ghost.into.and_then(|id| ids.iter().position(|&i| i == id))
            {
                ghost.rank = ease(ghost.rank, target as f32, dt);
            }
        }
        self.ghosts.retain(|ghost| ghost.fade > 0.0);
        !self.looks.is_empty() || !self.ghosts.is_empty()
    }
}
