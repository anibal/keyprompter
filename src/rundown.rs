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
                        section: section.title.clone(),
                        cue: card.cue.clone(),
                        body: deck::parse_body(&card.body),
                    })
                })
                .collect(),
        }
    }
}
