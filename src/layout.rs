//! Each card is laid out once, at full size. Smaller copies of a card draw
//! the same lines at a smaller size, so a card reads the same from upcoming
//! to current to past.

use gpui_kit::*;

use crate::deck::{Block, Slide, Span, SpanStyle};

/// Line height as a multiple of font size.
pub const LINE_HEIGHT: f32 = 1.5;
pub const CUE_RATIO: f32 = 1.4;
/// Body lines that fit under a one-line cue at the default size.
pub const BODY_LINES: f32 = 4.0;
/// Each text-size step scales both sizes by this factor.
pub const STEP: f32 = 1.1;
pub const MIN_STEP: i32 = -6;
pub const MAX_STEP: i32 = 8;

/// The two text sizes shared by every card, in points at full card size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeScale {
    pub cue: f32,
    pub body: f32,
}

impl TypeScale {
    /// The largest sizes that fit one cue line and `BODY_LINES` body lines in
    /// `content_h`, then moved `step` steps up or down.
    pub fn new(content_h: f32, step: i32) -> Self {
        let body = content_h / (LINE_HEIGHT * (CUE_RATIO + BODY_LINES))
            * STEP.powi(step.clamp(MIN_STEP, MAX_STEP));
        Self {
            cue: body * CUE_RATIO,
            body,
        }
    }

    pub fn indent(&self) -> f32 {
        self.body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    None,
    /// The first line of a bullet.
    Bullet,
    /// A later line of a bullet, indented under its text.
    Hang,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub mark: Mark,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardLayout {
    pub cue: Vec<String>,
    pub body: Vec<Line>,
}

/// How much of a card shows. When anything is hidden, the last body row
/// holds the `+N lines` badge instead of text, so the badge never covers a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fit {
    pub cue: usize,
    pub body: usize,
    pub hidden: usize,
}

impl CardLayout {
    pub fn fit(&self, scale: TypeScale, content_h: f32) -> Fit {
        let rows =
            |room: f32, size: f32| (room / (size * LINE_HEIGHT) + 1e-3).floor().max(0.0) as usize;
        let badge = scale.body * LINE_HEIGHT;
        let cue = if self.cue.len() <= rows(content_h, scale.cue) {
            self.cue.len()
        } else {
            rows(content_h - badge, scale.cue)
        };
        let body_rows = rows(content_h - cue as f32 * scale.cue * LINE_HEIGHT, scale.body);
        let total = self.cue.len() + self.body.len();
        let body = if cue == self.cue.len() && self.body.len() <= body_rows {
            self.body.len()
        } else {
            body_rows.saturating_sub(1).min(self.body.len())
        };
        Fit {
            cue,
            body,
            hidden: total - cue - body,
        }
    }
}

/// Cuts styled text into lines at the byte offsets where each later line
/// starts, dropping the spaces a line ends or starts with.
pub fn split(spans: &[Span], starts: &[usize]) -> Vec<Vec<Span>> {
    let mut lines = vec![Vec::new()];
    let mut offset = 0;
    let mut next = starts.iter().copied().peekable();
    for span in spans {
        let mut rest = span.text.as_str();
        let mut at = offset;
        while let Some(&start) = next.peek()
            && start < at + rest.len()
        {
            let (head, tail) = rest.split_at(start - at);
            push(lines.last_mut().unwrap(), head, span.style);
            lines.push(Vec::new());
            at = start;
            rest = tail;
            next.next();
        }
        push(lines.last_mut().unwrap(), rest, span.style);
        offset += span.text.len();
    }
    for line in &mut lines {
        if let Some(first) = line.first_mut() {
            first.text = first.text.trim_start().to_string();
        }
        if let Some(last) = line.last_mut() {
            last.text = last.text.trim_end().to_string();
        }
        line.retain(|s| !s.text.is_empty());
    }
    lines
}

fn push(line: &mut Vec<Span>, text: &str, style: SpanStyle) {
    if !text.is_empty() {
        line.push(Span {
            text: text.to_string(),
            style,
        });
    }
}

pub fn runs(spans: &[Span], base: &Font, color: Hsla, code_bg: Hsla) -> Vec<TextRun> {
    spans
        .iter()
        .map(|s| {
            let mut font = base.clone();
            let mut background_color = None;
            match s.style {
                SpanStyle::Plain => {}
                SpanStyle::Bold => font.weight = FontWeight::BOLD,
                SpanStyle::Italic => font.style = FontStyle::Italic,
                SpanStyle::Code => {
                    font.family = "Menlo".into();
                    background_color = Some(code_bg);
                }
            }
            TextRun {
                len: s.text.len(),
                font,
                color,
                background_color,
                underline: None,
                strikethrough: None,
            }
        })
        .collect()
}

/// Byte offsets where GPUI starts each wrapped line after the first. Sorted
/// and kept inside the text, so right-to-left glyph order can't break `split`.
fn line_starts(window: &Window, spans: &[Span], base: &Font, size: f32, width: f32) -> Vec<usize> {
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    let len = text.len();
    let runs = runs(spans, base, Hsla::default(), Hsla::default());
    let Ok(lines) =
        window
            .text_system()
            .shape_text(text.into(), px(size), &runs, Some(px(width)), None)
    else {
        return Vec::new();
    };
    let mut starts: Vec<usize> = lines
        .iter()
        .flat_map(|line| {
            line.wrap_boundaries
                .iter()
                .map(|b| line.unwrapped_layout.runs[b.run_ix].glyphs[b.glyph_ix].index)
        })
        .filter(|&start| start > 0 && start < len)
        .collect();
    starts.sort_unstable();
    starts.dedup();
    starts
}

pub fn layout(slide: &Slide, scale: TypeScale, width: f32, window: &Window) -> CardLayout {
    let base = window.text_style().font();
    let cue_font = Font {
        weight: FontWeight::BOLD,
        ..base.clone()
    };
    let cue = vec![Span {
        text: slide.cue.clone(),
        style: SpanStyle::Plain,
    }];
    let cue = split(
        &cue,
        &line_starts(window, &cue, &cue_font, scale.cue, width),
    )
    .into_iter()
    .map(|line| line.into_iter().map(|s| s.text).collect())
    .collect();
    let mut body = Vec::new();
    for block in &slide.body {
        let (spans, width, first, rest) = match block {
            Block::Bullet(spans) => (spans, width - scale.indent(), Mark::Bullet, Mark::Hang),
            Block::Para(spans) => (spans, width, Mark::None, Mark::None),
        };
        let starts = line_starts(window, spans, &base, scale.body, width);
        body.extend(
            split(spans, &starts)
                .into_iter()
                .enumerate()
                .map(|(i, spans)| Line {
                    mark: if i == 0 { first } else { rest },
                    spans,
                }),
        );
    }
    CardLayout { cue, body }
}
