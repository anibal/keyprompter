use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::deck::{self, Deck, Slide};

pub const FORMAT: u32 = 1;

/// A rundown the app owns. Markdown is only how one comes in and goes out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rundown {
    pub format: u32,
    pub title: String,
    pub imported_from: Option<PathBuf>,
    pub sections: Vec<Section>,
    /// Never reused, so an id that undo brings back can't belong to another card.
    #[serde(default)]
    pub next_id: u64,
}

/// Only the first section can be untitled: it holds the cards before any `#`.
/// A section exists only while it has cards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub title: Option<String>,
    pub cards: Vec<Card>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub id: CardId,
    pub cue: String,
    /// The lines under the cue, in the Markdown subset `deck::parse_body` reads.
    pub body: String,
}

/// Stays with a card through edits and moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CardId(pub u64);

impl Rundown {
    /// `#` starts a section and `##` a card. Lines before the first card of a
    /// section belong to no card and are dropped.
    pub fn import(markdown: &str, title: impl Into<String>) -> Self {
        let mut sections: Vec<Section> = Vec::new();
        let mut next = 0;
        for line in markdown.lines() {
            let line = line.trim();
            match deck::heading(line) {
                Some((1, text)) => sections.push(Section {
                    title: Some(text.to_string()),
                    cards: Vec::new(),
                }),
                Some((2, text)) => {
                    if sections.is_empty() {
                        sections.push(Section {
                            title: None,
                            cards: Vec::new(),
                        });
                    }
                    next += 1;
                    sections.last_mut().unwrap().cards.push(Card {
                        id: CardId(next),
                        cue: text.to_string(),
                        body: String::new(),
                    });
                }
                _ => {
                    if let Some(card) = sections.last_mut().and_then(|s| s.cards.last_mut()) {
                        card.body.push_str(line);
                        card.body.push('\n');
                    }
                }
            }
        }
        sections.retain(|section| !section.cards.is_empty());
        for card in sections.iter_mut().flat_map(|s| &mut s.cards) {
            card.body = card.body.trim().to_string();
        }
        Self {
            format: FORMAT,
            title: title.into(),
            imported_from: None,
            sections,
            next_id: next + 1,
        }
    }

    pub fn export(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            if let Some(title) = &section.title {
                out += &format!("# {title}\n\n");
            }
            for card in &section.cards {
                out += &format!("## {}\n", card.cue);
                if !card.body.is_empty() {
                    out += &card.body;
                    out.push('\n');
                }
                out.push('\n');
            }
        }
        out
    }

    /// The cards as the strip shows them, in order.
    pub fn deck(&self) -> Deck {
        Deck {
            slides: self
                .sections
                .iter()
                .flat_map(|section| {
                    section.cards.iter().map(|card| Slide {
                        id: card.id,
                        section: section.title.clone(),
                        cue: card.cue.clone(),
                        body: deck::parse_body(&card.body),
                    })
                })
                .collect(),
        }
    }
}

/// Which neighbour of a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Before,
    After,
}

/// A change to one card, named by id so it stays valid across other edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// A copy goes right next to the card, on the given side.
    Duplicate(CardId, Side),
    /// The next card's cue becomes a bullet and its body follows.
    MergeWithNext(CardId),
    /// Past the first card of a section, a card joins the end of the section before.
    MoveBefore(CardId),
    /// Past the last card of a section, a card joins the start of the section after.
    MoveAfter(CardId),
    /// Refused for the only card, so there is always one to edit.
    Delete(CardId),
    /// Replaces the cue and the body, both trimmed. Refused for an empty cue.
    SetText(CardId, String, String),
}

impl Rundown {
    pub fn cards(&self) -> impl Iterator<Item = &Card> {
        self.sections.iter().flat_map(|s| &s.cards)
    }

    pub fn card_ids(&self) -> Vec<CardId> {
        self.cards().map(|c| c.id).collect()
    }

    fn position(&self, id: CardId) -> Option<(usize, usize)> {
        self.sections.iter().enumerate().find_map(|(s, section)| {
            section
                .cards
                .iter()
                .position(|c| c.id == id)
                .map(|c| (s, c))
        })
    }

    fn new_id(&mut self) -> CardId {
        let id = CardId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Applies `edit` and returns the card to show afterwards, or `None`
    /// and no change when the edit doesn't apply.
    pub fn apply(&mut self, edit: Edit) -> Option<CardId> {
        let ids = self.card_ids();
        let focus = match edit {
            Edit::Duplicate(id, side) => {
                let (s, c) = self.position(id)?;
                let copy = Card {
                    id: self.new_id(),
                    ..self.sections[s].cards[c].clone()
                };
                let new = copy.id;
                let at = match side {
                    Side::Before => c,
                    Side::After => c + 1,
                };
                self.sections[s].cards.insert(at, copy);
                new
            }
            Edit::MergeWithNext(id) => {
                let next = *ids.get(ids.iter().position(|&i| i == id)? + 1)?;
                let (ns, nc) = self.position(next)?;
                let gone = self.sections[ns].cards.remove(nc);
                let (s, c) = self.position(id)?;
                let body = &mut self.sections[s].cards[c].body;
                let cue = if gone.cue.is_empty() {
                    String::new()
                } else {
                    format!("- {}", gone.cue)
                };
                let joined = [body.as_str(), &cue, &gone.body]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                *body = joined;
                id
            }
            Edit::MoveBefore(id) => {
                let (s, c) = self.position(id)?;
                if c > 0 {
                    self.sections[s].cards.swap(c, c - 1);
                } else {
                    if s == 0 {
                        return None;
                    }
                    let moved = self.sections[s].cards.remove(0);
                    self.sections[s - 1].cards.push(moved);
                }
                id
            }
            Edit::MoveAfter(id) => {
                let (s, c) = self.position(id)?;
                if c + 1 < self.sections[s].cards.len() {
                    self.sections[s].cards.swap(c, c + 1);
                } else {
                    if s + 1 == self.sections.len() {
                        return None;
                    }
                    let moved = self.sections[s].cards.remove(c);
                    self.sections[s + 1].cards.insert(0, moved);
                }
                id
            }
            Edit::Delete(id) => {
                let at = ids.iter().position(|&i| i == id)?;
                let focus = *ids
                    .get(at + 1)
                    .or(at.checked_sub(1).and_then(|p| ids.get(p)))?;
                let (s, c) = self.position(id)?;
                self.sections[s].cards.remove(c);
                focus
            }
            Edit::SetText(id, cue, body) => {
                let (cue, body) = (cue.trim(), body.trim());
                let (s, c) = self.position(id)?;
                let card = &mut self.sections[s].cards[c];
                if cue.is_empty() || (card.cue == cue && card.body == body) {
                    return None;
                }
                card.cue = cue.to_string();
                card.body = body.to_string();
                id
            }
        };
        self.sections.retain(|section| !section.cards.is_empty());
        Some(focus)
    }
}
